//
use eyre::Result;
use openvm_circuit::arch::instructions::{
    exe::{FnBounds, VmExe},
    instruction::{DebugInfo, Instruction},
    program::Program,
};
use openvm_native_recursion::hints::Hintable;
use openvm_sdk::{
    F, SC, Sdk, StdIn,
    config::{AppConfig, SdkVmConfig},
    fs::read_object_from_file,
};
use openvm_stark_sdk::openvm_stark_backend::proof::Proof;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::LazyLock,
};

const DEFAULT_SEGMENT_SIZE: usize = (1 << 22) - 1000;

static DATA_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data"));

pub fn get_sdk_chunk() -> Result<Sdk> {
    get_sdk(include_str!("../data/openvm_chunk.toml"))
}

pub fn get_sdk_batch() -> Result<Sdk> {
    get_sdk(include_str!("../data/openvm_batch.toml"))
}

/// Get or initialize the SDK lazily
fn get_sdk(toml_str: &str) -> Result<Sdk> {
    println!("Initializing SDK...");

    let mut app_config: AppConfig<SdkVmConfig> = toml::from_str(toml_str)?;
    let segmentation_limits = &mut app_config.app_vm_config.system.config.segmentation_limits;

    segmentation_limits.max_trace_height = DEFAULT_SEGMENT_SIZE as u32;
    segmentation_limits.max_cells = 1_200_000_000_usize; // For 24G vram

    Ok(Sdk::new(app_config)?)
}

/// Read input data from input_task.bin
pub fn get_input_data(middle_path: impl AsRef<Path>) -> Result<StdIn> {
    let working_path = DATA_PATH.join(middle_path);
    let data_path = working_path.join("input_task.bin");
    let bytes = fs::read(&data_path)?;
    let (input_bytes, sz) =
        bincode::decode_from_slice::<Vec<Vec<u8>>, _>(&bytes, bincode::config::standard())
            .map_err(|err| {
                eyre::eyre!("Failed to deserialize input bytes from {data_path:?}: {err}")
            })?;
    println!("read input ({sz} bytes)");
    let mut stdin = StdIn::default();

    for witness in &input_bytes {
        stdin.write_bytes(witness);
    }

    let agg_proof_path = working_path.join("agg_proofs.bin");
    if let Ok(bytes) = fs::read(&agg_proof_path) {
        let mut read_pos = bytes.as_slice();
        while let Ok((input_proofs, sz)) = bincode::serde::decode_from_slice::<Vec<Proof<SC>>, _>(
            read_pos,
            bincode::config::standard(),
        ) {
            if sz == 0 {
                break;
            }
            println!("read aggregated proof ({sz} bytes)");
            read_pos = &read_pos[sz..];
            let streams = input_proofs[0].write();
            for s in &streams {
                stdin.write_field(s);
            }
        }
    }

    Ok(stdin)
}

/// Wrapper around [`openvm_sdk::fs::read_exe_from_file`].
pub fn read_legacy_exe<P: AsRef<Path>>(path: P) -> eyre::Result<VmExe<F>> {
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

#[cfg(test)]
mod exe_metered_cost;

#[cfg(test)]
mod core_dump;

use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt};

/// Setup test environment
pub fn setup_logger() {
    static LOG_INIT: std::sync::OnceLock<()> = std::sync::OnceLock::new();

    LOG_INIT.get_or_init(|| {
        let fmt_layer = tracing_subscriber::fmt::layer()
            .pretty()
            .with_span_events(FmtSpan::CLOSE);

        let filters = tracing_subscriber::filter::Targets::new()
            .with_target("openvm_", tracing_subscriber::filter::LevelFilter::INFO);

        tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .with(fmt_layer)
            .with(filters)
            .try_init()
            .unwrap();
    });
}
