use ame::client::build_ame_client;
use ame::client::AmeClient;
use ame::client::AmeCtrl;
use ame::grpc::*;
use ame::web::{Header, HeaderProps};
use ame::AmeServiceClientCfg;
use leptos::html::Input;
use leptos::{
    component, create_action, create_local_resource, create_node_ref, create_rw_signal,
    create_signal, create_signal_from_stream, log, provide_context, use_context, view, Children,
    For, ForProps, IntoView, ReadSignal, RwSignal, Scope, SignalGet, SignalSet, SignalWith,
    WriteSignal,
};

#[allow(unused)]
use leptos::tracing;

use leptos_meta::*;
use leptos_router::*;
use tonic::Request;
use tonic_web_wasm_client::Client;

pub fn gen_client(_endpoint: String) -> AmeClient {
    let ame_endpoint = "http://ame.local:32117".to_string();
    build_ame_client(AmeServiceClientCfg {
        id_token: None,
        endpoint: ame_endpoint,
        disable_tls_cert_check: false,
    })
    .unwrap()
}

async fn list_project_srcs(_: ()) -> Vec<String> {
    let mut client = gen_client("test".to_string());

    let res = client
        .list_resource(tonic::Request::new(ResourceListParams {
            params: Some(resource_list_params::Params::ProjectSourceListParams(
                ProjectSourceListParams {},
            )),
        }))
        .await;

    res.unwrap()
        .into_inner()
        .ids
        .into_iter()
        .map(|id| match id.id {
            Some(resource_id::Id::ProjectSrcId(ProjectSourceId { name })) => name,
            None => todo!(),
        })
        .collect()
}

async fn get_project_src(id: String) -> ProjectSourceCfg {
    let mut client = gen_client("".to_string());
    let src_cfg = client
        .get_project_src_cfg(Request::new(ProjectSourceId { name: id.clone() }))
        .await
        .unwrap();

    src_cfg.into_inner()
}

async fn watch_project_src(cx: Scope, id: String) -> ReadSignal<Option<ProjectSourceStatus>> {
    let ctrl: AmeCtrl<Client> = use_context(cx).unwrap();
    create_signal_from_stream(
        cx,
        ctrl.watch_project_src(ProjectSourceId { name: id.clone() })
            .await
            .unwrap(),
    )
}

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context(cx);
    provide_context(cx, AmeCtrl::new(gen_client("".to_string())));

    view! {
        cx,
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="primer" href="https://unpkg.com/@primer/css@^20.2.4/dist/primer.css"/>
        <Stylesheet id="leptos" href="/pkg/start-axum.css"/>

        <Header/>
        <Router >
         <div class="Layout">
          <nav class="SideNav border Layout-sidebar" style="max-width: 360px">
            <A class="SideNav-item" href="projectsrcs">"Project Sources"</A>
            <A class="SideNav-item" href="secrets">"Secrets"</A>
         </nav>
            <main class="Layout-main ">
                <Routes>
                    <Route path="projectsrcs" view=move |cx| view! { cx, <ProjectSrcs/> } />
                    <Route path="projectsrcs/new" view=move |cx| view! { cx, <CreateProjectSrc/> }/>
                    <Route path="projectsrcs/:name" view=move |cx| view! { cx, <ProjectSrcPage/> }/>
                </Routes>
            </main>
            </div>
        </Router>
    }
}

#[component]
fn project_src_page(cx: Scope) -> impl IntoView {
    let params = use_params_map(cx);
    let project_src = create_local_resource(
        cx,
        move || params.with(|p| p.get("name").cloned().unwrap_or_default()),
        move |id| async move {
            let mut ctrl: AmeCtrl<Client> = AmeCtrl::new(gen_client("".to_string()));
            ctrl.client
                .get_project_src_cfg(Request::new(id.into()))
                .await
                .map(|r| r.into_inner())
                .ok()
        },
    );

    view! {
                cx,
                {move || view!{cx, <ProjectSrcForm src=project_src.read(cx).unwrap_or(None)  submit_text=Some("Update project source".to_string()) name={params.get().get("name").cloned()} />
            }
        }
    }
}

#[component]
fn create_project_src(cx: Scope) -> impl IntoView {
    view! {
        cx,
        <ProjectSrcForm src=None name=None submit_text=None/>
    }
}

#[component]
fn project_src_form(
    cx: Scope,
    name: Option<String>,
    src: Option<ProjectSourceCfg>,
    submit_text: Option<String>,
) -> impl IntoView {
    let repo_ref = create_node_ref::<Input>(cx);
    let username_ref = create_node_ref::<Input>(cx);

    let (secret, set_secret) = create_signal::<Option<String>>(cx, None);

    let show = create_rw_signal(cx, false);
    let message = create_rw_signal(cx, "".to_string());

    let create_project_src = create_action(
        cx,
        move |input: &(Scope, ProjectSourceCfg, Option<String>)| {
            let (cx, cfg, name) = input.to_owned();

            async move {
                if let Some(name) = name {
                    let id: ProjectSourceId = name.into();

                    use_context::<AmeCtrl<Client>>(cx)
                        .expect("expect ame client to be present")
                        .client
                        .update_project_src(Request::new(ProjectSrcPatchRequest {
                            id: Some(id.clone()),
                            cfg: Some(cfg),
                        }))
                        .await
                        .map(|_| id)
                } else {
                    use_context::<AmeCtrl<Client>>(cx)
                        .expect("expect ame client to be present")
                        .client
                        .create_project_src(Request::new(cfg))
                        .await
                        .map(|res| res.into_inner())
                }
            }
        },
    );

    let project_src_id = create_project_src.value();

    let navigate = use_navigate(cx);

    let (current_repo, current_username) =
        if let Some(ProjectSourceCfg { git: Some(git_cfg) }) = src {
            (
                git_cfg.repository,
                git_cfg.username.unwrap_or("".to_string()),
            )
        } else {
            ("my.git.repository".to_string(), "".to_string())
        };

    view! {cx,
              <div class="Box Box--spacious col-6 mx-auto mt-6">

                  <div class="Box-header">
      <h3 class="Box-title">
      "Project Source"
      </h3>
    </div>

              <form class="Box=body m-4 " on:submit=move |e| {
                  e.prevent_default();
                  let repo = repo_ref.get().expect("repo to exist").value();
                  let username = username_ref.get().expect("username to exist").value();

                  let username = if !username.is_empty() { Some(username) } else {None};
                  let secret= secret.get();

                  create_project_src.dispatch((cx, ProjectSourceCfg::new_git_source(repo, username, secret), name.clone()));

                  }>
              <div class="form-group">
                <div class="form-group-header">
                  <label for="git-repository">"Git repository"</label>
                </div>
                <div class="form-group-body">
                  <input node_ref=repo_ref class="form-control" type="text" value=current_repo id="git-repository" />
                </div>
              </div>

              <div class="form-group">
            <div class="form-group-header">
              <label for="username-input"> "Git Username" </label>
            </div>
            <div class="form-group-body">
              <input
              node_ref=username_ref
                class="form-control"
                type="text"
                value=current_username
                id="username-input"
                aria-describedby="username-input-validation"
              />
            </div>
          </div>

          <SecretSelector secret_sig=set_secret/>

          <PrimerButton class="m-2".to_string() r#type="submit".to_string() variant=PrimerButtonVariant::Primary> {submit_text.unwrap_or("Create Project Source".to_string())} </PrimerButton>

            </form>

            { move || {
                          if let Some(Ok(_)) = project_src_id.get() {
                              let _ = navigate("/projectsrcs", Default::default());
                          } else if let Some(Err(err)) = project_src_id.get() {
                            show.set(true);
                            message.set(err.message().to_string());
                          }
                  }

            }
      </div>
        <Toast variant=ToastVariant::Danger message=message.read_only() show=show class="position-fixed bottom-0 left-0".to_string()/>
                }
}

#[component]
fn secret_selector(cx: Scope, secret_sig: WriteSignal<Option<String>>) -> impl IntoView {
    let secrets = create_local_resource(
        cx,
        move || (),
        |_| async move {
            let mut ctrl: AmeCtrl<Client> = AmeCtrl::new(gen_client("".to_string()));

            ctrl.client
                .list_secrets(Request::new(Empty {}))
                .await
                .unwrap()
                .into_inner()
        },
    );

    let secret_node = create_node_ref(cx);

    view! {cx,
        <div class="form-group">
          <div class="form-group-header">
            <label for="example-select">"Select secret"</label>
          </div>
          <div class="form-group-body">{move || {
              if let Some(secrets) = secrets.read(cx) {
                  if secrets.secrets.is_empty() {
                      view!{cx,

          <select class="form-select" id="secret-select" disabled=true >
              <option>"No secrets available" </option>
              </select>
                  }
                  } else {
                      view!{cx,
          <select class="form-select" node_ref=secret_node id="secret-select" on:change={let secrets = secrets.clone(); move|_|{
              let selected_index = if let Some(select) = secret_node.get()  {
                   select.selected_index()
              } else {
                  return;
              };

              if selected_index > 0 {
              secret_sig.set(Some(secrets.secrets[selected_index as usize - 1].key.clone()));
              } else {

              secret_sig.set(None);
              }

          }
          }
               >
        <option> "No selection" </option>
            {

                secrets.secrets.into_iter().map(|s|{
                view!{cx,
                <option value=s.key.clone()> {s.key} </option>
                }
                }).collect::<Vec<_>>()

            }
            </select>

                  }
              }} else {
                  view!{cx,
          <select class="form-select" id="secret-select" disabled=true >
              "Loading secrets..."
              </select>
              }
          }}}
          <PrimerButton r#type="button".to_string() variant=PrimerButtonVariant::Primary class="m-2".to_string()> "New Secret" </PrimerButton>
          </div>
          </div>

    }
}

/// Renders the home page of your application.
#[component]
fn ProjectSrcs(cx: Scope) -> impl IntoView {
    let srcs = create_local_resource(cx, || (), list_project_srcs);

    view! {
        cx,

        <div class="mt-4 container-xl">

        <div class="d-flex flex-items-end mb-md-4 flex-justify-between mr-4">
         <div class="d-flex flex-justify-start flex-auto width-full " role="search">

         <form class=" ml-0 width-full subnav-search">
            <input class="form-control width-full input-contrast subnav-search-input" placeholder="filter project sources" type="text"/>
         </form>

         </div>
         <A href="new">
         <PrimerButton class="ml-3".to_string() variant=PrimerButtonVariant::Primary> "New" </PrimerButton>
         </A>

        </div>

        {move || {
                    view! {cx,

        <div class="d-flex flex-row flex-wrap">
            <For
            each=move || srcs.read(cx).unwrap_or(vec![])
            key=|s| s.clone()
            view=move |cx, s: String | {
            view! {cx,  <ProjectSrc id={s}/> }
            } />
    </div>
                    }.into_any()
                 }}

    </div>
        }
}

#[allow(clippy::redundant_clone)]
#[component]
pub fn ProjectSrc(cx: Scope, id: String) -> impl IntoView {
    let src = create_local_resource(
        cx,
        {
            let id = id.clone();
            move || id.clone()
        },
        get_project_src,
    );

    let (deleted, set_deleted) = create_signal(cx, false);

    let delete = {
        let id = id.clone();
        move |_| {
            let delete_btn = create_action(cx, |input: &(String, Scope)| {
                let (id, cx) = input;
                let ctrl: AmeCtrl<Client> = use_context(*cx).unwrap();

                ctrl.delete_project_src(ProjectSourceId { name: id.clone() })
            });
            delete_btn.dispatch((id.clone(), cx));
            set_deleted.set(true);
        }
    };

    view! { cx,
               {
                   move || {
                            if let Some(ProjectSourceCfg{git: Some(GitProjectSource { repository, ..})}) = src.read(cx) {
                                view! {cx,
          <div class="Box mr-4 mt-4 col-md-4 col-sm-4 col-lg-3 flex-shrink-0" class:v-hidden=move || deleted.get() >
               <div class="Box-header flex-shrink-0 ">
                    <div class="d-flex flex-row flex-items-center flex-wrap">
      <GitBranchIcon size=PrimerSize::Small class="flex-shrink-0".to_string()/>
      <h3 class="Box-title flex-shrink-0">
      {
          let rep = repository.clone();
          rep.split('/').last().unwrap().to_owned()
               }
      </h3>
    <ProjectSrcState class="float-right".to_string() id={id.clone()}/>
                    </div>
    </div>
    <div class="Box-body">
        {repository.split("//").last().unwrap().to_owned()}
    </div>
    <div class="Box-footer text-right" >
    <PrimerButton variant=PrimerButtonVariant::Danger on:click=delete.clone() class="mr-2".to_string()  > "Delete" </PrimerButton>
    <A href={id.clone()}>
        <PrimerButton variant=PrimerButtonVariant::Primary class="mr-2".to_string()>  "View" </PrimerButton>
    </A>
    </div>
              </div>

                                }
                            } else {
                                view! {cx,

          <div class="Box m-4">
               <div class="Box-header">
      <h3 class="Box-title">
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" width="16" height="16"><path d="M9.5 3.25a2.25 2.25 0 1 1 3 2.122V6A2.5 2.5 0 0 1 10 8.5H6a1 1 0 0 0-1 1v1.128a2.251 2.251 0 1 1-1.5 0V5.372a2.25 2.25 0 1 1 1.5 0v1.836A2.493 2.493 0 0 1 6 7h4a1 1 0 0 0 1-1v-.628A2.25 2.25 0 0 1 9.5 3.25Zm-6 0a.75.75 0 1 0 1.5 0 .75.75 0 0 0-1.5 0Zm8.25-.75a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5ZM4.25 12a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5Z"></path></svg>
    "Waiting"
      </h3>
    </div>
    <div class="Box-body">
      "https://github.com/TeaInSpace/ame-demo.git"
    </div>
    <div class="Box-footer text-right">
    <PrimerButton variant=PrimerButtonVariant::Danger class="btn btn-danger mr-2".to_string()> "Delete" </PrimerButton>
    <button class="btn btn-primary"> "Edit" </button>
    </div>
              </div>
                                }
                            }
                        }

               }

      }
}

#[component]
pub fn ProjectSrcState(cx: Scope, id: String, #[prop(optional)] class: String) -> impl IntoView {
    let status = create_local_resource(cx, move || cx, {
        move |_| watch_project_src(cx, id.clone())
    });

    view! {cx, <div>
        {
            move || {
    let class = format!("ml-2 {class}");
                let status_sig = status.read(cx);
                if let Some(status_sig)= status_sig {
                    log!("from some");

                    let inner_sig = status_sig.get();

                    if inner_sig.is_none() {
                        return view! {
                            cx,
                                        <Label class=class variant={LabelVariant::Accent}><DotFill size={PrimerSize::Small} class="anim-pulse".to_string()/>   "Pending"</Label>
                        }
                    }

                    view! { cx,
                        {match ProjectSourceState::from_i32(status_sig.get().unwrap().state) {
                                Some(ProjectSourceState::Synchronized) => view! { cx,
                                        <Label class=class variant={LabelVariant::Primary}><DotFill size={PrimerSize::Small} class="anim-pulse".to_string()/>   "Healthy"</Label>
                                    },
                                Some(ProjectSourceState::Pending) =>
                                    view! {
                                        cx,
                                        <Label class=class variant={LabelVariant::Accent}><DotFill size={PrimerSize::Small} class="anim-pulse".to_string()/>   "Pending"</Label>
                                    }
                        ,
                                _ =>
                                    view! {
                                        cx,
                                        <Label class=class variant={LabelVariant::Danger}><DotFill size={PrimerSize::Small} class="anim-pulse".to_string()/>   "Degraded"</Label>
                                    }
                            }
                    }
                    }
                } else {
                    view! {
                                            cx,
                                        <Label class=class variant={LabelVariant::Accent}><DotFill size={PrimerSize::Small} class="anim-pulse".to_string()/>   "Pending"</Label>

                    }
                }
            }
        }
                </div>

    }
}

pub enum PrimerSize {
    Small,
    Large,
}

impl PrimerSize {
    pub fn pixels(&self) -> i32 {
        match self {
            PrimerSize::Small => 16,
            PrimerSize::Large => 24,
        }
    }
}

pub enum LabelVariant {
    Primary,
    Accent,
    Danger,
}

impl LabelVariant {
    pub fn variant_class(&self) -> String {
        match self {
            LabelVariant::Primary => "Label--success",
            LabelVariant::Accent => "Label--accent",
            LabelVariant::Danger => "Label--danger",
        }
        .to_string()
    }
}

#[component]
pub fn label(
    cx: Scope,
    variant: LabelVariant,
    #[prop(optional)] class: String,
    children: Children,
) -> impl IntoView {
    view! {cx,
        <span class=format!("Label {} {}", variant.variant_class(), class)> {children(cx)}  </span>
    }
}

#[component]
pub fn dot_fill(cx: Scope, size: PrimerSize, #[prop(optional)] class: String) -> impl IntoView {
    view! {cx, <svg class=class style="vertical-align: text-bottom;"
        fill="currentColor"
        xmlns="http://www.w3.org/2000/svg"
        viewBox=format!("0 0 {} {}",size.pixels(), size.pixels()) width=size.pixels()
        height=size.pixels()>
        <path d="M8 4a4 4 0 1 1 0 8 4 4 0 0 1 0-8Z"/>
    </svg>
    }
}

#[component]
pub fn git_branch_icon(
    cx: Scope,
    size: PrimerSize,
    #[prop(optional)] class: String,
) -> impl IntoView {
    view! {cx,
    <svg xmlns="http://www.w3.org/2000/svg" class=class viewBox=format!("0 0 {} {}", size.pixels(), size.pixels()) width=size.pixels() height=size.pixels()><path d="M9.5 3.25a2.25 2.25 0 1 1 3 2.122V6A2.5 2.5 0 0 1 10 8.5H6a1 1 0 0 0-1 1v1.128a2.251 2.251 0 1 1-1.5 0V5.372a2.25 2.25 0 1 1 1.5 0v1.836A2.493 2.493 0 0 1 6 7h4a1 1 0 0 0 1-1v-.628A2.25 2.25 0 0 1 9.5 3.25Zm-6 0a.75.75 0 1 0 1.5 0 .75.75 0 0 0-1.5 0Zm8.25-.75a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5ZM4.25 12a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5Z"></path></svg>

       }
}

pub enum OticonVariant {
    Search,
}

impl OticonVariant {
    fn path(&self) -> &str {
        match self {
            OticonVariant::Search => { "M10.68 11.74a6 6 0 0 1-7.922-8.982 6 6 0 0 1 8.982 7.922l3.04 3.04a.749.749 0 0 1-.326 1.275.749.749 0 0 1-.734-.215ZM11.5 7a4.499 4.499 0 1 0-8.997 0A4.499 4.499 0 0 0 11.5 7Z"
            }
        }
    }
}

#[component]
pub fn oticon(
    cx: Scope,
    size: PrimerSize,
    #[prop(optional)] class: String,
    variant: OticonVariant,
) -> impl IntoView {
    view! {cx,
    <svg xmlns="http://www.w3.org/2000/svg" class=format!("oticon {class}") viewBox=format!("0 0 {} {}", size.pixels(), size.pixels()) width=size.pixels() height=size.pixels()><path d=variant.path()></path></svg>

       }
}

pub enum PrimerButtonVariant {
    Primary,
    Danger,
}

impl PrimerButtonVariant {
    pub fn class(&self) -> &str {
        match self {
            PrimerButtonVariant::Primary => "btn-primary",
            PrimerButtonVariant::Danger => "btn-danger",
        }
    }
}

#[component]
pub fn primer_button(
    cx: Scope,
    variant: PrimerButtonVariant,
    #[prop(optional)] class: String,
    #[prop(optional)] r#type: String,
    children: Children,
) -> impl IntoView {
    view! {cx,
        <button type=r#type class=format!("btn {} {}", variant.class(), class)> {children(cx)} </button>
    }
}

pub enum ToastVariant {
    Danger,
}

impl ToastVariant {
    pub fn icon(&self, cx: Scope) -> impl IntoView {
        view! {cx,
        <span class="Toast-icon">
              <svg width="14" height="16" viewBox="0 0 14 16" class="octicon octicon-stop" aria-hidden="true">
                <path
                  fill-rule="evenodd"
                  d="M10 1H4L0 5v6l4 4h6l4-4V5l-4-4zm3 9.5L9.5 14h-5L1 10.5v-5L4.5 2h5L13 5.5v5zM6 4h2v5H6V4zm0 6h2v2H6v-2z"
                />
              </svg>
            </span>
               }
    }

    pub fn class(&self) -> String {
        "Toast--error".to_string()
    }
}

#[component]
pub fn toast(
    cx: Scope,
    variant: ToastVariant,
    message: ReadSignal<String>,
    #[prop(optional)] class: String,
    show: RwSignal<bool>,
) -> impl IntoView {
    let toast_ref = create_node_ref(cx);
    let (hide, set_hide) = create_signal(cx, false);

    view! {cx,
    <div class=format!("p-1  {class}") class:v-hidden={move || !show.get() && !hide.get()} class:anim-fade-in={move || show.get()} class:anim-fade-out={move ||  !show.get()} node_ref=toast_ref>
      <div class=format!("Toast {}", variant.class())>
                {variant.icon(cx)}
        <span class="Toast-content">{move || message.get()}</span>
            <button class="Toast-dismissButton" on:click=move|_|{
                show.set(false);
                set_hide.set(true);
            }>
      <svg width="12" height="16" viewBox="0 0 12 16" class="octicon octicon-x" aria-hidden="true">
        <path
          fill-rule="evenodd"
          d="M7.48 8l3.75 3.75-1.48 1.48L6 9.48l-3.75 3.75-1.48-1.48L4.52 8 .77 4.25l1.48-1.48L6 6.52l3.75-3.75 1.48 1.48L7.48 8z"
        />
      </svg>
    </button>
      </div>
    </div>

        }
}
