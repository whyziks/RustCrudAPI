use uuid::Uuid;
use tokio::io::AsyncWriteExt;
use axum::{Router,routing::delete,routing::get,routing::post,extract::Json,http::StatusCode,extract::Path,routing::patch};
use serde::{Deserialize,Serialize};
//MAIN PROCESS
#[tokio::main]
async fn main()->Result<(),std::io::Error>{
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
async fn tasks_get()->Json<Vec<Task>>{
	let mut vec:Vec<Task> = Vec::new();
	let mut out = String::new();
	let mut db = tokio::fs::read_to_string("db.txt").await.unwrap();
	for line in db.lines(){
		if line.trim().is_empty(){continue}
		let mut iter = line.trim().split("|");
		vec.push(Task{id:iter.next().unwrap().to_string(),
			title:iter.next().unwrap().to_string(),
			completed:iter.next().unwrap().parse::<bool>().unwrap()
		});
	}
	Json(vec)
	
}
async fn task_create(Json(payload):Json<TaskCreate>)->StatusCode{

	let id = Uuid::new_v4();
	let mut write = format!("{}|{}|{}\n",id,payload.title,payload.completed);
	let mut file = tokio::fs::OpenOptions::new()
		.write(true)
		.append(true)
		.create(true)
		.open("db.txt")
		.await
		.unwrap();
	file.write_all(write.as_bytes()).await.unwrap();
	return StatusCode::CREATED;
}
async fn get_task(Path(id):Path<String>)->Result<Json<Task>,StatusCode>{
	let mut db = match tokio::fs::read_to_string("db.txt").await{
		Ok(content)=>content,
		Err(_)=>return Err(StatusCode::INTERNAL_SERVER_ERROR),
	};
	for line in db.lines(){
		let mut iter = line.trim().split("|");
		let id1 = iter.next().unwrap();
		if id1==id{
			let mut iter2 = line.trim().split("|");
			let task =Task{id:iter2.next().unwrap().to_string(),title:iter2.next().unwrap().to_string(),completed:iter2.next().unwrap().parse::<bool>().unwrap(),};
			return Ok(Json(task))
		}
	}
	return Err(StatusCode::NOT_FOUND)
}
async fn delete_task(Path(id):Path<String>)->StatusCode{
	let mut text = tokio::fs::read_to_string("db.txt").await.unwrap();
	let mut text_out = String::new();
	let mut found = false;
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
		return StatusCode::NO_CONTENT
	}
	else{return StatusCode::NOT_FOUND}
}	
async fn patch_task(Path(id):Path<String>,Json(payload):Json<TaskPatch>)->StatusCode{
	let mut text = tokio::fs::read_to_string("db.txt").await.unwrap();
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
