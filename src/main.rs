use sqlx::sqlite::{SqlitePoolOptions,SqliteConnectOptions};
use sqlx::{SqlitePool,Executor,Row};
use uuid::Uuid;
use tracing::instrument;
use axum::{Router,routing::delete,routing::get,routing::post,extract::Json,extract::State,http::StatusCode,extract::Path,routing::patch};
use serde::{Deserialize,Serialize};
//MAIN PROCESS
#[tokio::main]
async fn main()->Result<(),std::io::Error>{
        //DB CONNECTION
        let url = "sqlite:tasks.db";
        let options = SqliteConnectOptions::new()
            .filename("tasks.db")
            .create_if_missing(true);
        let pool = SqlitePool::connect_with(options).await.unwrap();
        pool.execute("
            CREATE TABLE IF NOT EXISTS tasks(
                id TEXT,
                title TEXT NOT NULL,
                completed BOOLEAN NOT NULL DEFAULT FALSE)").await.unwrap();
        //ERROR TRACING
	tracing_subscriber::fmt()
		.with_max_level(tracing::Level::INFO)
		.init();
	tracing::info!("Starting server");
        //SETTING AXUM
        let state = AppState{db:pool};
	let app = Router::new()
		.route("/tasks",get(tasks_get).post(task_create))
		.route("/health",get(|| async{"Server is running"}))
                .route("/db",get(db_get).post(db_access))
		.route("/tasks/{id}",get(get_task).patch(patch_task).delete(delete_task))
                .with_state(state);
	let listener = tokio::net::TcpListener::bind("127.0.0.1:1488").await.unwrap();
	axum::serve(listener,app).await.unwrap();
	Ok(())
}
//STRUCTURES
#[derive(Clone,Debug)]
struct AppState{
    db:SqlitePool,
}
#[derive(Deserialize,Serialize)]
struct TaskCreate{
	title: String,
	completed: bool,
}
#[derive(Deserialize,Serialize)]
struct Task{
	id:String,
	title:String,
	completed:bool,
}
#[derive(Deserialize,Serialize)]
struct TaskPatch{
	title: Option<String>,
	completed: Option<bool>,
}
//REQUEST-FUNCTIONS
#[instrument(skip_all)]
async fn tasks_get(State(state):State<AppState>)->Result<Json<Vec<Task>>,StatusCode>{
        tracing::info!("got get tasks request");
        let answer = sqlx::query("SELECT * FROM tasks")
            .fetch_all(&state.db)
            .await
            .unwrap();
        let vec:Vec<Task> = answer.iter().map(|row| Task{id:row.get("id"),title:row.get("title"),completed:row.get("completed")}).collect();
        tracing::info!("done");
	Ok(Json(vec))
	
}
#[instrument(skip_all)]
async fn task_create(State(state):State<AppState>,Json(payload):Json<TaskCreate>)->StatusCode{
	tracing::info!("Started creating a task");
	let id = Uuid::new_v4();
        let result = sqlx::query("INSERT INTO tasks (id,title,completed) VALUES (?, ?, ?)")
            .bind(id.to_string())
            .bind(payload.title)
            .bind(payload.completed)
            .execute(&state.db)
            .await
            .unwrap();
	tracing::info!("Task has been written in db");
	return StatusCode::CREATED;
}
#[instrument(skip_all)]
async fn get_task(Path(id):Path<String>,State(state):State<AppState>)->Result<Json<Task>,StatusCode>{
	tracing::info!("Got get_task request");
        let answer = sqlx::query("SELECT * FROM tasks WHERE id=?")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .unwrap();
        match answer{
            Some(x)=>{tracing::info!("done");return Ok(Json(Task{id:x.get("id"),title:x.get("title"),completed:x.get("completed")}))},
            None =>{tracing::warn!("task not found"); return Err(StatusCode::NOT_FOUND)}
        };
}
#[instrument(skip_all)]
async fn delete_task(Path(id):Path<String>,State(state):State<AppState>)->StatusCode{
	tracing::info!("got delete_task request");
        let answer = match sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(id)
            .execute(&state.db)
            .await{
                Ok(_)=>{tracing::info!("task was successfully deleted");return StatusCode::NO_CONTENT},
                Err(_)=>{tracing::warn!("task was not found");return StatusCode::NOT_FOUND}
            };
}
#[instrument(skip_all)]
async fn patch_task(Path(id):Path<String>,State(state):State<AppState>,Json(payload):Json<TaskPatch>)->StatusCode{
	tracing::info!("got patch_task request");
        let answer = match sqlx::query("SELECT title, completed FROM tasks WHERE id = ?")
            .bind(id.clone())
            .fetch_optional(&state.db)
            .await{
                Ok(Some(x))=>{tracing::info!("found line");x},
                Ok(None)=>{tracing::info!("line does not exist");return StatusCode::NOT_FOUND},
                Err(err)=>{tracing::error!("database error");return StatusCode::INTERNAL_SERVER_ERROR}
            };
        let task:TaskCreate = TaskCreate{title:answer.get("title"),completed:answer.get("completed")};
        let title = payload.title.unwrap_or(task.title);
        let completed = payload.completed.unwrap_or(task.completed);
        match sqlx::query("UPDATE tasks SET title = ?, completed = ? WHERE id = ?")
            .bind(title)
            .bind(completed)
            .bind(id)
            .execute(&state.db)
            .await{
                Ok(_)=>{tracing::info!("task has been successfully updated");return StatusCode::OK},
                Err(_)=>{tracing::info!("error while trying to access DB");return StatusCode::INTERNAL_SERVER_ERROR},
            };
}
#[instrument(skip_all)]
// doesnt work yet
// async fn db_access(State(state):State<AppState>,body:String)->StatusCode{
//     let bodystr = body.as_str();
//     let answer = match sqlx::query(bodystr)
//         .execute(&state.db)
//         .await{
//             Ok(_)=>return StatusCode::OK,
//             Err(_)=>return StatusCode::INTERNAL_SERVER_ERROR,
//         };
// }
// async fn db_get(State(state):State<AppState>,body:String)->String{
//     let bodystr  = body.as_str();
//     let answer:Vec<String> = sqlx::query_scalar(bodystr)
//         .fetch_all(&state.db)
//         .await
//         .unwrap();
//     let mut ans = String::new();
//     if answer.len()!=1{
//         for line in answer.iter(){
//             ans = format!("{}{}\n",ans,line);
//         }
//         return ans;
//     }
//     return answer[0].to_string();
//
// }
