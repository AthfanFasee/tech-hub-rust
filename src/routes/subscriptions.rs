use actix_web::{web, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct SubscriptionData {
    email: String,
    name: String,
}

pub async fn subscribe(payload: web::Json<SubscriptionData>, pool: web::Data<PgPool>) -> HttpResponse {
  match sqlx::query!(
      r#"
        INSERT INTO users (id, name, email)
	    VALUES ($1, $2, $3)
	   "#,
        Uuid::new_v4(),
        payload.name,
        payload.email,
  )
      .execute(pool.get_ref())
      .await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
        println!("Failed to execute query: {e}");
        HttpResponse::InternalServerError().finish()
    }
    }


}