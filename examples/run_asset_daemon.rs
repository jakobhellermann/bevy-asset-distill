use distill::daemon::AssetDaemon;

fn main() {
    let (handle, _) = AssetDaemon::default().run();
    handle.join().unwrap();
}
