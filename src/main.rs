fn main() {
    #[cfg(feature = "hot_reload")]
    ridiculous_bevy_hot_reloading::dyn_load_main("main", None);
    #[cfg(not(feature = "hot_reload"))]
    lib_SOL::app();
}
