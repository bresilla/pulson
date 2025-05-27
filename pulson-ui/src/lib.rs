// pulson-ui/src/lib.rs

mod components;

use components::{Dashboard, Settings};
use gloo_net::http::Request;
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, SubmitEvent};
use yew::prelude::*;
use yew_router::prelude::*;

/// ----
/// 1) Routes
/// ----
#[derive(Routable, PartialEq, Clone, Debug)]
enum Route {
    #[at("/login")]
    Login,
    #[at("/")]
    Dashboard,
    #[at("/settings")]
    Settings,
    #[not_found]
    #[at("/404")]
    NotFound,
}

/// switch must take Route by value
fn switch(routes: Route) -> Html {
    match routes {
        Route::Login => html! { <Login /> },
        Route::Dashboard => html! { <Dashboard /> },
        Route::Settings => html! { <Settings /> },
        Route::NotFound => html! { <h1>{ "404 – page not found" }</h1> },
    }
}

/// ----
/// 2) Login Component
/// ----
#[function_component(Login)]
fn login() -> Html {
    let username = use_state(String::new);
    let password = use_state(String::new);
    let error = use_state(|| None::<String>);
    let navigator = use_navigator().unwrap();

    let onsubmit = {
        let username = username.clone();
        let password = password.clone();
        let error = error.clone();
        let navigator = navigator.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let user = (*username).clone();
            let pass = (*password).clone();
            let error = error.clone();
            let navigator = navigator.clone();

            spawn_local(async move {
                #[derive(Serialize)]
                struct Payload<'a> {
                    username: &'a str,
                    password: &'a str,
                }

                let payload = Payload {
                    username: &user,
                    password: &pass,
                };

                // .body(...) returns Result<Request, Error>, so unwrap()
                let request = Request::post("/api/account/login")
                    .header("Content-Type", "application/json")
                    .body(serde_json::to_string(&payload).unwrap())
                    .unwrap();

                let resp = request.send().await;

                match resp {
                    Ok(r) if r.status() == 200 => {
                        #[derive(Deserialize)]
                        struct Resp {
                            token: String,
                        }
                        match r.json::<Resp>().await {
                            Ok(data) => {
                                LocalStorage::set("pulson_token", data.token).unwrap();
                                navigator.push(&Route::Dashboard);
                            }
                            Err(err) => {
                                error.set(Some(format!("Invalid JSON: {}", err)));
                            }
                        }
                    }
                    Ok(r) => {
                        let txt = r.text().await.unwrap_or_default();
                        error.set(Some(format!("Login failed: {}", txt)));
                    }
                    Err(err) => {
                        error.set(Some(format!("Network error: {}", err)));
                    }
                }
            });
        })
    };

    html! {
        <form {onsubmit} class="login-form" style="max-width:300px; margin:50px auto;">
            <h2>{ "Pulson Login" }</h2>
            if let Some(err) = &*error {
                <p style="color:red;">{ err }</p>
            }
            <div>
                <label for="user">{ "Username:" }</label>
                <input
                    id="user"
                    value={(*username).clone()}
                    oninput={Callback::from(move |e: InputEvent| {
                        let input: HtmlInputElement = e.target_unchecked_into();
                        username.set(input.value());
                    })}
                />
            </div>
            <div>
                <label for="pass">{ "Password:" }</label>
                <input
                    id="pass"
                    type="password"
                    value={(*password).clone()}
                    oninput={Callback::from(move |e: InputEvent| {
                        let input: HtmlInputElement = e.target_unchecked_into();
                        password.set(input.value());
                    })}
                />
            </div>
            <button type="submit">{ "Log in" }</button>
        </form>
    }
}

/// ----
/// 3) Dashboard - now using the full component
/// ----
// Dashboard component is now in components/dashboard.rs

/// ----
/// 4) App entry
/// ----
#[function_component(App)]
fn app() -> Html {
    html! {
        <BrowserRouter>
            <Switch<Route> render={switch} />
        </BrowserRouter>
    }
}

#[wasm_bindgen(start)]
pub fn run_app() {
    yew::Renderer::<App>::new().render();
}
