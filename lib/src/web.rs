use leptos::*;

#[component]
pub fn ProjectSrc(cx: Scope, title: String) -> impl IntoView {
    view! { cx,
          <div class="Box">
               <div class="Box-header">
      <h3 class="Box-title">
      {title}
      </h3>
    </div>
    <div class="Box-body">
      "Box body"
    </div>
    <div class="Box-footer">
      "Box footer"
    </div>
              </div>

      }
}

#[component]
pub fn Header(cx: Scope) -> impl IntoView {
    view! {
            cx,
            <div class="Header">
      <div class="Header-item">
        <a href="#" class="Header-link f4 d-flex flex-items-center">
          <span>"AME"</span>
        </a>
      </div>
      <div class="Header-item">
        <input type="search" class="form-control Header-input" />
      </div>
      <div class="Header-item mr-0">
      </div>
    </div>
        }
}
