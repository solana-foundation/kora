mod routes;

use routes::transfer;

let app = Router::new()
    .merge(transfer::routes());

