use std::sync::Once;

static START: Once = Once::new();

pub fn start_fake_plc_global() {
    START.call_once(|| {
        std::thread::spawn(|| {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(ethernetip::fake_plc::run_fake_plc())
                .unwrap();
        });
    });
}
