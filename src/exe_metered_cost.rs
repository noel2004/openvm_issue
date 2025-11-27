use std::{
    fs,
    path::{Path, PathBuf},
    sync::LazyLock,
};
use eyre::Result;
use openvm_sdk::{
    F, Sdk, StdIn, 
    fs::read_object_from_file,
    config::{AppConfig, SdkVmConfig},
};
use openvm_circuit::arch::instructions::{
    exe::{FnBounds, VmExe},
    instruction::{DebugInfo, Instruction},
    program::Program,
};

const DEFAULT_SEGMENT_SIZE: usize = (1 << 22) - 1000;

static DATA_PATH : LazyLock<PathBuf> = LazyLock::new(||{
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
});

/// Get or initialize the SDK lazily
fn get_sdk() -> Result<Sdk> {

    println!("Initializing SDK...");
    
    let mut app_config : AppConfig<SdkVmConfig> = toml::from_str(include_str!("../data/openvm.toml"))?;
    let segmentation_limits =
        &mut app_config.app_vm_config.system.config.segmentation_limits;

    segmentation_limits.max_trace_height = DEFAULT_SEGMENT_SIZE as u32;
    segmentation_limits.max_cells = 1_200_000_000_usize; // For 24G vram

    Ok(Sdk::new(app_config)?)
}

fn get_input_data(middle_path: impl AsRef<Path>) -> Result<StdIn> {
    let data_path = DATA_PATH.join(middle_path).join("input_task.bin");
    let bytes = fs::read(&data_path)?;
    let (input_bytes, sz) = bincode::decode_from_slice::<Vec<Vec<u8>>, _>(&bytes, bincode::config::standard())
        .map_err(|err| eyre::eyre!("Failed to deserialize StdIn from {data_path:?}: {err}"))?;
    println!("read input ({sz} bytes)");
    let mut stdin = StdIn::default();

    for witness in &input_bytes {
        stdin.write_bytes(witness);
    }

    Ok(stdin)
}


/// Wrapper around [`openvm_sdk::fs::read_exe_from_file`].
fn read_legacy_exe<P: AsRef<Path>>(path: P) -> eyre::Result<VmExe<F>> {
    if let Ok(r) = read_object_from_file(&path) {
        return Ok(r);
    }

    println!("loading vmexe failed, trying old format..");

    /// Executable program for OpenVM.
    #[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
    #[serde(bound(serialize = "F: Serialize", deserialize = "F: Deserialize<'de>"))]
    pub struct OldProgram<F> {
        #[serde(deserialize_with = "deserialize_instructions_and_debug_infos")]
        pub instructions_and_debug_infos: Vec<Option<(Instruction<F>, Option<DebugInfo>)>>,
        pub step: u32,
        pub pc_base: u32,
    }
    #[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
    #[serde(bound(
        serialize = "F: serde::Serialize",
        deserialize = "F: std::cmp::Ord + serde::Deserialize<'de>"
    ))]
    pub struct OldVmExe<F> {
        /// Program to execute.
        pub program: OldProgram<F>,
        /// Start address of pc.
        pub pc_start: u32,
        /// Initial memory image.
        pub init_memory: std::collections::BTreeMap<(u32, u32), F>,
        /// Starting + ending bounds for each function.
        pub fn_bounds: FnBounds,
    }
    use serde::{Deserialize, Deserializer, Serialize};

    #[allow(clippy::type_complexity)]
    fn deserialize_instructions_and_debug_infos<'de, F: Deserialize<'de>, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<Option<(Instruction<F>, Option<DebugInfo>)>>, D::Error> {
        let (inst_data, total_len): (Vec<(Instruction<F>, u32)>, u32) =
            Deserialize::deserialize(deserializer)?;
        let mut ret: Vec<Option<(Instruction<F>, Option<DebugInfo>)>> = Vec::new();
        ret.resize_with(total_len as usize, || None);
        for (inst, i) in inst_data {
            ret[i as usize] = Some((inst, None));
        }
        Ok(ret)
    }

    let old_exe: OldVmExe<F> = read_object_from_file(&path)?;
    use openvm_stark_sdk::openvm_stark_backend::p3_field::FieldAlgebra;
    use openvm_stark_sdk::openvm_stark_backend::p3_field::PrimeField32;
    let exe = VmExe::<F> {
        program: Program::<F> {
            instructions_and_debug_infos: old_exe.program.instructions_and_debug_infos,
            pc_base: old_exe.program.pc_base,
        },
        pc_start: old_exe.pc_start,
        init_memory: old_exe
            .init_memory
            .into_iter()
            .map(|(k, v)| {
                assert!(v < F::from_canonical_u32(256u32));
                (k, v.as_canonical_u32() as u8)
            })
            .collect(),
        fn_bounds: old_exe.fn_bounds,
    };
    Ok(exe)
}

#[test]
fn test_execute_cost() -> Result<()> {
    let sdk = get_sdk()?;
    let exe : VmExe<F> = read_legacy_exe(DATA_PATH.join("new").join("app.vmexe"))?;
    let (public_values, (_cost, total_cycle)) = sdk
        .execute_metered_cost(exe, get_input_data("new")?)?;

    println!("public_values after guest execution {:?}, cycle {}", public_values, total_cycle);
    if public_values.iter().all(|x| *x == 0) {
        eyre::bail!("public_values are all 0s".to_string());
    }

    Ok(())
}

#[test]
fn test_execute_cost_legacy() -> Result<()> {
    let sdk = get_sdk()?;
    let exe : VmExe<F> = read_legacy_exe(DATA_PATH.join("legacy").join("app.vmexe"))?;
    let (public_values, (_cost, total_cycle)) = sdk
        .execute_metered_cost(exe, get_input_data("legacy")?)?;

    println!("public_values after guest execution {:?}, cycle {}", public_values, total_cycle);
    if public_values.iter().all(|x| *x == 0) {
        eyre::bail!("public_values are all 0s".to_string());
    }

    Ok(())
}
