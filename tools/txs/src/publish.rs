//! tools for publishing move package
//!
use anyhow::bail;
use diem::{common::types::MovePackageDir, move_tool::MAX_PUBLISH_PACKAGE_SIZE};
// use diem::common::types::CliError;
// use diem::common::types::TransactionSummary;
use diem_framework::{BuildOptions, BuiltPackage};
// use diem::move_tool::PublishPackage;
// use libra_types::type_extensions::client_ext::TransactionOptions;

use diem_types::transaction::TransactionPayload;

/// build the move package and create a transaction payload.
pub fn encode_publish_payload(move_options: &MovePackageDir) -> anyhow::Result<TransactionPayload> {
    let package_path = move_options.get_package_path()?;
    let options = BuildOptions {
        // NOTE: if the file includes a named address the build will fail.
        named_addresses: move_options.named_addresses(),
        ..Default::default()
    };

    let package = BuiltPackage::build(package_path, options)?;
    let compiled_units = package.extract_code();

    // Send the compiled module and metadata using the code::publish_package_txn.
    let metadata = package.extract_metadata()?;
    let payload = libra_cached_packages::libra_stdlib::code_publish_package_txn(
        bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
        compiled_units,
    );
    let size = bcs::serialized_size(&payload)?;
    println!("package size {} bytes", size);
    if size > MAX_PUBLISH_PACKAGE_SIZE {
        bail!(
            "The package is larger than {} bytes ({} bytes)! To lower the size \
            you may want to include less artifacts via `--included-artifacts`.",
            MAX_PUBLISH_PACKAGE_SIZE,
            size
        );
    }
    Ok(payload)
}
