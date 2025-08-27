crate::globals::addons::export_addon_modules!();

//fn init_crash_on_err(clb: Box<dyn FnOnce() -> Result<()>>) {
//    if let Err(err) = clb() {
//        console_error!(err);
//        CRASH_HANDLE.with_borrow_mut(|crash_handle| crash_handle.crashed = true);
//    }
//}
