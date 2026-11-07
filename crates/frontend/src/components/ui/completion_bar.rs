use leptos::prelude::*;

/// Thin `done/total` progress bar with an optional `done/total` label.
///
/// Renders nothing but the track when there is nothing to complete, so callers
/// can use it for both full sections and single rows.
#[component]
pub fn CompletionBar(
    /// Number of completed entries.
    #[prop()]
    done: usize,
    /// Number of entries in total.
    #[prop()]
    total: usize,
    /// Show the `done/total` text next to the bar.
    #[prop(default = true)]
    show_text: bool,
    /// Extra classes for the wrapping row.
    #[prop(default = String::new(), into)]
    class: String,
) -> impl IntoView {
    let percent = if total == 0 {
        0.0
    } else {
        (done.min(total) as f64 / total as f64) * 100.0
    };

    view! {
        <div class=format!("flex items-center gap-2 {class}")>
            <div
                class="h-1.5 flex-1 rounded-full bg-base-300 overflow-hidden"
                role="progressbar"
                aria-valuemin="0"
                aria-valuemax=total
                aria-valuenow=done
            >
                <div
                    class="h-full bg-primary transition-[width] duration-300"
                    style=format!("width: {percent:.1}%")
                />
            </div>
            {show_text.then(|| {
                view! {
                    <span class="text-xs tabular-nums text-base-content/60">
                        {done} "/" {total}
                    </span>
                }
            })}
        </div>
    }
}
