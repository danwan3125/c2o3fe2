// use axum::{Router, routing::get, routing::post};

// pub struct AgentState //All agents and command history
// {
//     agents:Arc<tokio::sync::RwLock<HashMap<String,Agent>>>,

//     // tasks:Mutex<Vec<Task>>
// }
// pub fn op_router<S>(state:AgentState) -> Router<S>{
//     let op_routes=Router::new()
//     .route("/",get(health_check))
//     .route("/tasks/{id}",post(||async{}))
//     .route("/tasks/{id}",get(||async{}))
//     .route("/tasks",get(||async{}))
//     .route("/agents",get(||async{}))
//     .route("/agents/{id}",get(||async{}))
//     .with_state(state);
// }