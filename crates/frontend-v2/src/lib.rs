pub mod app;
pub mod components;
pub mod pages;
pub mod server_fns;
pub mod state;

pub use app::App;

/// HTML shell rendered by the server for every SSR request.
/// SSR-only: not compiled into the WASM bundle.
#[cfg(feature = "ssr")]
pub fn shell(options: leptos::config::LeptosOptions) -> impl leptos::IntoView {
    use leptos::prelude::*;

    view! {
        <!DOCTYPE html>
        <html lang="pl" data-theme="neon-night">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1.0, viewport-fit=cover"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <link rel="stylesheet" href="/pkg/kartoteka.css"/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

/// Entry point called by the browser after the WASM bundle loads.
/// Hydrate-only: not compiled into the server binary.
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
