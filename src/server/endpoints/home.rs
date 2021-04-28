use actix_web::{get, HttpResponse, Responder};

const HOME_HTML: &str = r"
<!doctype html>
<html lang='en'>
    <head>
        <style>
            .bg-dark {
                background-color: #272727;
            }
            .color-light {
                color: #bfbfbf;
            }
            .text-center {
                text-align: center;
            }
            .fs-24 {
                font-size: 24px;
            }
            .pt-20 {
                padding-top: 20px;
            }
        </style>
    </head>
    <body class='bg-dark color-light text-center fs-24 pt-20'>
        Bld server is running!
    </body>
</html>
";

#[get("/")]
async fn home() -> impl Responder {
    HttpResponse::Ok().body(HOME_HTML)
}
