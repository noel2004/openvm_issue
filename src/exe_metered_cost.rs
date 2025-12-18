use eyre::Result;
use openvm_sdk::F;

use super::*;

#[test]
fn test_execute_cost() -> Result<()> {
    let sdk = get_sdk_chunk()?;
    let exe: VmExe<F> = read_legacy_exe(DATA_PATH.join("new").join("app.vmexe"))?;
    let (public_values, (_cost, total_cycle)) =
        sdk.execute_metered_cost(exe, get_input_data("new")?)?;

    println!(
        "public_values after guest execution {:?}, cycle {}",
        public_values, total_cycle
    );
    if public_values.iter().all(|x| *x == 0) {
        eyre::bail!("public_values are all 0s".to_string());
    }

    Ok(())
}

#[test]
fn test_execute_cost_legacy() -> Result<()> {
    let sdk = get_sdk_chunk()?;
    let exe: openvm_circuit::arch::instructions::exe::VmExe<F> =
        read_legacy_exe(DATA_PATH.join("legacy").join("app.vmexe"))?;
    let (public_values, (_cost, total_cycle)) =
        sdk.execute_metered_cost(exe, get_input_data("legacy")?)?;

    println!(
        "public_values after guest execution {:?}, cycle {}",
        public_values, total_cycle
    );
    if public_values.iter().all(|x| *x == 0) {
        eyre::bail!("public_values are all 0s".to_string());
    }

    Ok(())
}
