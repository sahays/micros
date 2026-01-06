use askama::Template;
use axum::response::{Html, IntoResponse, Redirect};
use tower_sessions::Session;

#[derive(Template)]
#[template(path = "admin.html")]
pub struct AdminTemplate {}

pub async fn admin_dashboard_handler(session: Session) -> impl IntoResponse {
    let access_token: Option<String> = session.get("access_token").await.unwrap_or_default();
    if access_token.is_none() {
        return Redirect::to("/login").into_response();
    }

    let template = AdminTemplate {};
    template.into_response()
}

pub async fn service_list_fragment() -> impl IntoResponse {
    Html("
        <table class='w-full text-left'>
            <thead>
                <tr class='text-zinc-500 text-sm border-b border-white/5'>
                    <th class='pb-4 font-medium'>Service Name</th>
                    <th class='pb-4 font-medium'>Scopes</th>
                    <th class='pb-4 font-medium'>Created</th>
                    <th class='pb-4 font-medium'>Actions</th>
                </tr>
            </thead>
            <tbody class='text-sm'>
                <tr class='border-b border-white/5'>
                    <td class='py-4 font-medium'>Billing Service</td>
                    <td class='py-4'><span class='bg-blue-500/10 text-blue-400 px-2 py-0.5 rounded'>user:read</span></td>
                    <td class='py-4 text-zinc-500'>2026-01-05</td>
                    <td class='py-4 text-blue-400 hover:underline cursor-pointer'>Rotate Key</td>
                </tr>
            </tbody>
        </table>
    ").into_response()
}

pub async fn user_list_fragment() -> impl IntoResponse {
    Html(
        "
        <div class='grid grid-cols-1 md:grid-cols-2 gap-4'>
            <div class='p-4 bg-white/5 rounded-2xl flex justify-between items-center'>
                <div>
                    <p class='font-medium'>user@example.com</p>
                    <p class='text-xs text-zinc-500'>Verified • User</p>
                </div>
                <button class='text-red-400 hover:underline text-xs'>Lock</button>
            </div>
        </div>
    ",
    )
    .into_response()
}
