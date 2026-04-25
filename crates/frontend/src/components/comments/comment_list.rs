use kartoteka_shared::types::Comment;
use leptos::prelude::*;

#[component]
pub fn CommentList(
    comments: Vec<Comment>,
    current_user_id: String,
    on_delete: Callback<String>,
) -> impl IntoView {
    if comments.is_empty() {
        return view! {
            <p class="text-base-content/40 text-sm italic py-2">"Brak komentarzy."</p>
        }
        .into_any();
    }

    view! {
        <div class="flex flex-col gap-3">
            {comments
                .into_iter()
                .map(|comment| {
                    let is_assistant = comment.author_type == "assistant";
                    let is_mine = comment.user_id == current_user_id;
                    let comment_id_del = comment.id.clone();

                    view! {
                        <div class=move || {
                            if is_assistant {
                                "rounded-lg p-3 bg-secondary/10 border border-secondary/20"
                            } else {
                                "rounded-lg p-3 bg-base-200"
                            }
                        }>
                            <div class="flex items-center justify-between mb-1">
                                <span class="text-xs font-semibold text-base-content/60">
                                    {if is_assistant {
                                        view! { <span class="badge badge-secondary badge-xs mr-1">"AI"</span> }.into_any()
                                    } else {
                                        view! {}.into_any()
                                    }}
                                    {if is_assistant {
                                        comment.author_name.clone().unwrap_or_else(|| "Assistant".into())
                                    } else {
                                        "Ty".into()
                                    }}
                                </span>
                                <div class="flex items-center gap-2">
                                    <span class="text-xs text-base-content/40">{comment.created_at.clone()}</span>
                                    {if is_mine {
                                        let id = comment_id_del.clone();
                                        view! {
                                            <button
                                                type="button"
                                                class="btn btn-ghost btn-xs btn-circle text-error"
                                                on:click=move |_| on_delete.run(id.clone())
                                            >
                                                {"✕"}
                                            </button>
                                        }
                                        .into_any()
                                    } else {
                                        view! {}.into_any()
                                    }}
                                </div>
                            </div>
                            <p class="text-sm text-base-content whitespace-pre-wrap">{comment.content.clone()}</p>
                        </div>
                    }
                })
                .collect::<Vec<_>>()}
        </div>
    }
    .into_any()
}
