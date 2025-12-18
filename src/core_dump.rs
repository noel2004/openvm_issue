use eyre::Result;
use openvm_sdk::F;

use super::*;

#[test]
fn test_proving_core_dump() -> Result<()> {
    setup_logger();

    let sdk = get_sdk_batch()?;
    let exe: VmExe<F> = read_legacy_exe(DATA_PATH.join("core_dump").join("app.vmexe"))?;

    let proof = sdk.prover(exe)?.prove(get_input_data("core_dump")?)?;
    println!("proving finished, PI: {:?}", proof.user_public_values);

    Ok(())
}

#[test]
fn test_proving_execute_ok() -> Result<()> {
    setup_logger();

    let sdk = get_sdk_batch()?;
    let exe: VmExe<F> = read_legacy_exe(DATA_PATH.join("core_dump").join("app.vmexe"))?;

    let (public_values, (_cost, total_cycle)) =
        sdk.execute_metered_cost(exe, get_input_data("core_dump")?)?;

    println!(
        "public_values after guest execution {:?}, cycle {}",
        public_values, total_cycle
    );

    Ok(())
}
