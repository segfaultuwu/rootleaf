use crate::{fs::vfs, shell::commands::run_script};

const AUTORUN_PATHS: [&str; 2] = [
    "/ram/AUTORUN.BAT",
    "/disk1/AUTORUN.BAT",
];

pub fn init() {
    for &autorun in AUTORUN_PATHS.iter() {
        if vfs::exists(&autorun) {
            crate::print!("Autorun: found autorun script, executing...\n");
            run_script(autorun.as_bytes());
            return;
        }
    }
    crate::print!("Autorun: no autorun script found\n");
}