pub mod builder;
pub mod framework_cli;
pub mod release;
//////// 0L ///////
/// Returns the release bundle for the current code.
pub fn head_release_bundle() -> diem_framework::ReleaseBundle {
    release::ReleaseTarget::Head
        .load_bundle()
        .expect("release build failed")
}
