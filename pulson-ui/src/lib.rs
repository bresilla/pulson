use wasm_bindgen::prelude::*;
use yew::prelude::*;
use yew::Renderer;

/// Your root component
#[function_component(App)]
fn app() -> Html {
    html! {
        <div>
            <h1>{ "Pulson Dashboard" }</h1>
            <p>{ "Hello from Yew!" }</p>
        </div>
    }
}

/// This is invoked automatically in the browser when the module loads
#[wasm_bindgen(start)]
pub fn run_app() {
    // If you have a `<div id="root"></div>` in your index.html and
    // want to mount into that, you could do:
    //
    // let document = web_sys::window().unwrap().document().unwrap();
    // let root = document.get_element_by_id("root").unwrap();
    // Renderer::<App>::with_root(root).render();

    // Otherwise by default it will mount into <body>
    Renderer::<App>::new().render();
}
