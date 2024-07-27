#[cfg(feature = "hot_reload")]
use ridiculous_bevy_hot_reloading::dyn_load_main;

fn main() {
    // Everything needs to be in the library for the TypeIds to be consistent between builds.

    // Copies library file before running so the original can be overwritten
    // Only needed if using bevy_dylib. Otherwise this could just be `lib_make_hot_bevy::main();`
    #[cfg(feature = "hot_reload")]
    dyn_load_main("main", None);
    #[cfg(not(feature = "hot_reload"))]
    lib_bevy_jam_5::app();
}
