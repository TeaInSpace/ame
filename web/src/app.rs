use self::proto::ame_service_client::AmeServiceClient;
use self::proto::TaskIdentifier;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use tonic_web_wasm_client::Client;

pub mod proto {
    tonic::include_proto!("ame.v1");
}

pub async fn gen_client(_endpoint: String) {
    let ame_endpoint = "http://ame.local:30966".to_string();
    let wasm_client = Client::new(ame_endpoint);

    println!("gen client");

    let mut ame_client = AmeServiceClient::new(wasm_client);
    let _res = ame_client
        .get_task(TaskIdentifier {
            name: "mytask6wt6x".to_string(),
        })
        .await;
}

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context(cx);

    create_action(cx, |task: &String| gen_client(task.clone())).dispatch("endpoint".to_string());

    view! {
        cx,

        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/start-axum.css"/>

        // sets the document title
        <Title text="Welcome to Leptos"/>

        // content for this welcome page
        <Router>
            <main>
                <Routes>
                    <Route path="" view=|cx| view! { cx, <HomePage/> }/>
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage(cx: Scope) -> impl IntoView {
    // Creates a reactive value to update the button
    let (count, set_count) = create_signal(cx, 0);
    let on_click = move |_| set_count.update(|count| *count += 1);

    view! { cx,
        <h1>"Welcome to Leptos!"</h1>
        <button on:click=on_click>"Click Me: " {count}</button>
    }
}
