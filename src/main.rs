use uuid::Uuid;
use tracing::instrument;
use std::time::Instant;
use tokio::io::AsyncWriteExt;
use axum::{Router,routing::delete,routing::get,routing::post,extract::Json,http::StatusCode,extract::Path,routing::patch};
use serde::{Deserialize,Serialize};
//MAIN PROCESS
#[tokio::main]
async fn main()->Result<(),std::io::Error>{
	tracing_subscriber::fmt()
		.with_max_level(tracing::Level::INFO)
		.init();
	tracing::info!("Starting server");
	let app = Router::new()
		.route("/tasks",get(tasks_get).post(task_create))
		.route("/health",get(|| async{"Server is running"}))
		.route("/tasks/{id}",get(get_task).patch(patch_task).delete(delete_task));
	let listener = tokio::net::TcpListener::bind("127.0.0.1:1488").await.unwrap();
	axum::serve(listener,app).await.unwrap();
	Ok(())
}
//STRUCTURES
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
#[instrument]
async fn tasks_get()->Result<Json<Vec<Task>>,StatusCode>{
	tracing::info!("Started reading db");
	let mut db = match tokio::fs::read_to_string("db.txt").await{
		Ok(x) => x,
		Err(_) => {tracing::error!("FAILED TO READ DB"); return Err(StatusCode::INTERNAL_SERVER_ERROR)}
	};
	tracing::info!("db read correctly, starting validating db");
	let mut tasks = match validate_db(&db){
		Ok(x)=>x,
		Err(x)=>{tracing::error!("DB IS NOT IN PROPER CONDITION"); return Err(StatusCode::INTERNAL_SERVER_ERROR)},
	};
	tracing::info!("db validated, sending answer");
	Ok(Json(tasks))
	
}
#[instrument(skip_all)]
async fn task_create(Json(payload):Json<TaskCreate>)->StatusCode{
	tracing::info!("Started creating a task");
	let id = Uuid::new_v4();
	let mut write = format!("{}|{}|{}\n",id,payload.title,payload.completed);
	tracing::info!("Started reading db");
	let mut file = tokio::fs::OpenOptions::new()
		.write(true)
		.append(true)
		.create(true)
		.open("db.txt")
		.await
		.unwrap();
	tracing::info!("DB successfully read");
	file.write_all(write.as_bytes()).await.unwrap();
	tracing::info!("Task has been written in db");
	return StatusCode::CREATED;
}
#[instrument(skip_all)]
async fn get_task(Path(id):Path<String>)->Result<Json<Task>,StatusCode>{
	tracing::info!("Got get_task request, started reading db");
	let mut db = match tokio::fs::read_to_string("db.txt").await{
		Ok(content)=>content,
		Err(_)=>{tracing::error!("DB NOT FOUND");return Err(StatusCode::INTERNAL_SERVER_ERROR)},
	};
	tracing::info!("db was read successfully");
	for line in db.lines(){
		let mut iter = line.trim().split("|");
		let id1 = iter.next().unwrap();
		if id1==id{
			let mut iter2 = line.trim().split("|");
			let task =Task{id:iter2.next().unwrap().to_string(),title:iter2.next().unwrap().to_string(),completed:iter2.next().unwrap().parse::<bool>().unwrap(),};
			tracing::info!("task is found");
			return Ok(Json(task))
		}
	}
	tracing::error!("task is not found");
	return Err(StatusCode::NOT_FOUND)
}
#[instrument(skip_all)]
async fn delete_task(Path(id):Path<String>)->StatusCode{
	tracing::info!("got delete_task request, started reading db");
	let mut text = match tokio::fs::read_to_string("db.txt").await{
		Ok(content)=>content,
		Err(_)=>{tracing::error!("DB NOT FOUND");return StatusCode::INTERNAL_SERVER_ERROR},
	};
	tracing::info!("db was read successfully");
	let mut text_out = String::new();
	let mut found = false;
	tracing::info!("finding task in db");
	for line in text.lines(){
		let mut iter = line.trim().split("|");
		let mut id1 = iter.next().unwrap().to_string();
		if id==id1{
			found=true;
			continue;
		}
		text_out = format!("{}{}\n",text_out,line);
	}
	tokio::fs::write("db.txt",text_out).await.unwrap();
	if found{
		tracing::info!("task found && deleted");
		return StatusCode::NO_CONTENT
	}
	else{tracing::error!("task was not found");return StatusCode::NOT_FOUND}
}
#[instrument(skip_all)]
async fn patch_task(Path(id):Path<String>,Json(payload):Json<TaskPatch>)->StatusCode{
	tracing::info!("got patch_task request,started reading db");
	let mut text = match tokio::fs::read_to_string("db.txt").await{
		Ok(content)=>content,
		Err(_)=>{tracing::error!("DB NOT FOUND");return StatusCode::INTERNAL_SERVER_ERROR},
	};
	tracing::info!("db was successfully read");
	let mut text_out = String::new();
	let mut found = false;
	if (payload.title.is_none() && payload.completed.is_none())|| payload.title.as_deref().unwrap_or("cruto").trim()==""{
		return StatusCode::BAD_REQUEST;
	}
	for line in text.lines(){
		let mut iter = line.trim().split("|");
		let mut id1 = iter.next().unwrap().to_string();
		let mut title = iter.next().unwrap().to_string();
		let mut completed:bool = iter.next().unwrap().parse().unwrap();
		if id==id1{
			found=true;
			let title_out = payload.title.as_deref().unwrap_or(&title);
			let completed_out = match payload.completed{
				Some(x) => x,
				None => completed,
			};
			text_out = format!("{}{}|{}|{}\n",text_out,id,title_out,completed_out);
			continue;
		}
		text_out = format!("{}{}\n",text_out,line);
	}
	if found{
		tokio::fs::write("db.txt",text_out).await.unwrap();
		return StatusCode::OK
	}
	else{return StatusCode::NOT_FOUND}
}
fn validate_db(db:&str)->Result<Vec<Task>,String>{
	let mut vec_out:Vec<Task> = Vec::new();
	for (i, line) in db.lines().enumerate(){
		let mut vec:Vec<&str> = line.trim().split("|").collect();
		if line.trim().is_empty(){
			continue;
		}
		if vec.len()!=3{
			return Err(format!("line {i}: LESS OR MORE FIELDS IN LINE THAN EXPECTED"));
		}
		match Uuid::parse_str(vec[0]){
			Ok(_)=>"id is correct",
			Err(_)=> return Err(format!("line {i}: ONE OF THE LINES CONTAINS INCORRECT ID")),		
		};
		if vec[1].trim() == ""{
			return Err(format!("line {i}: ONE OF THE LINES CONTAINS BLANK NAME"))
		}
		if vec[2] != "true" && vec[2]!= "false"{
			return Err(format!("line {i}: ONE OF THE LINES CONTAINS NON-BOOLEAN EXPRESSION IN COMPLETED FIELD"));
		}
		vec_out.push(Task{id:vec[0].to_string(),title:vec[1].to_string(),completed:vec[2].parse::<bool>().unwrap()});
	}
	return Ok(vec_out)
}
