use std::future::Future;

#[cfg(not(target_arch = "wasm32"))]
pub fn execute<F: Future<Output = ()> + Send + 'static>(f: F) {
    // this is stupid... use any executor of your choice instead
    std::thread::spawn(move || futures::executor::block_on(f));
}
#[cfg(target_arch = "wasm32")]
pub fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}

pub async fn sleep_ms(mills: u64) {
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::sleep(std::time::Duration::from_millis(mills));
    #[cfg(target_arch = "wasm32")]
    {
        // #[wasm_bindgen]
        pub fn sleep(ms: i32) -> js_sys::Promise {
            js_sys::Promise::new(&mut |resolve, _| {
                web_sys::window()
                    .unwrap()
                    .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
                    .unwrap();
            })
        }
        let promise = sleep(mills as i32);
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    }
}