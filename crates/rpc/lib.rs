pub mod routes;

use warp::Filter;

#[tokio::main]
async fn main() {
    let routes = routes::transfer::transfer_route();

    println!("Running on http://localhost:3030");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
