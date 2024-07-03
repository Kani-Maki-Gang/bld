use leptos::*;

#[component]
pub fn Login() -> impl IntoView {
    view! {
        <div class="w-full flex justify-center self-center">
            <div class="flex rounded-xl bg-slate-700 min-w-[1000px] p-[100px]">
                <img class="max-w-[400px] max-h-[400px]" src="logo.png"/>
                <div class="rounded-xl w-96 p-8 ml-24 bg-slate-800 flex flex-col">
                    <div class="flex-none text-3xl text-white">
                        "Simple and blazingly fast CI/CD"
                    </div>
                    <div class="grow mt-4 text-lg text-gray-500">
                        "Use the below button to redirect to your OIDC provider"
                    </div>
                    <a
                        class="h-[40px] w-full text-center rounded-lg p-2 focus:outline-none bg-indigo-600 hover:bg-indigo-700 focus:bg-indigo-700"
                        href="http://localhost:6080/v1/auth/web-client/start"
                    >
                        "Login"
                    </a>
                </div>
            </div>
        </div>
    }
}
