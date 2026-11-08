use crate::{
    AppState,
    auth::{AuthenticatedStaff, require_capability},
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct OptionValue {
    pub code: String,
    pub label: String,
    pub color: Option<String>,
    #[serde(default)]
    pub media_ids: Vec<Uuid>,
}
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct ProductOption {
    pub code: String,
    pub label: String,
    pub control: String,
    pub values: Vec<OptionValue>,
}
#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct VariantConfig {
    pub options: Vec<ProductOption>,
    pub color_photos: bool,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct VariantTemplate {
    pub id: Uuid,
    pub definition: ProductOption,
    pub revision: i64,
}
#[derive(Deserialize, ToSchema)]
pub struct MatrixVariant {
    pub id: Option<Uuid>,
    pub selections: BTreeMap<String, String>,
    pub price_minor: i64,
    pub available_quantity: i64,
    pub expected_quantity: Option<i64>,
    pub retired: bool,
}
#[derive(Deserialize, ToSchema)]
pub struct SaveMatrix {
    pub revision: i64,
    pub config: VariantConfig,
    pub variants: Vec<MatrixVariant>,
}
fn fail(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(json!({"error":{"code":"variant_configuration","message":message}})),
    )
        .into_response()
}
fn valid_code(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 8
        && s.bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
}
fn valid_option(o: &ProductOption) -> bool {
    let mut codes = BTreeSet::new();
    let mut labels = BTreeSet::new();
    valid_code(&o.code)
        && !o.label.trim().is_empty()
        && o.label.len() <= 80
        && matches!(o.control.as_str(), "swatch" | "buttons" | "select")
        && !o.values.is_empty()
        && o.values.len() <= 100
        && o.values.iter().all(|v| {
            valid_code(&v.code)
                && codes.insert(&v.code)
                && labels.insert(v.label.trim().to_lowercase())
                && v.media_ids.len() <= 100
                && !v.label.trim().is_empty()
                && v.label.len() <= 80
                && (o.control != "swatch"
                    || v.color.as_ref().is_some_and(|c| {
                        c.len() == 7
                            && c.starts_with('#')
                            && c[1..].bytes().all(|b| b.is_ascii_hexdigit())
                    }))
        })
}
#[utoipa::path(get,path="/api/admin/variant-templates",responses((status=200,body=Vec<VariantTemplate>)),tag="admin catalog")]
pub async fn templates(State(s): State<AppState>, a: AuthenticatedStaff) -> Response {
    if let Err(e) = require_capability(&a, "catalog.read") {
        return e.into_response();
    }
    let Some(p) = s.database else {
        return fail(StatusCode::SERVICE_UNAVAILABLE, "Database unavailable");
    };
    match sqlx::query_as::<_,(Uuid,Value,i64)>("SELECT id,definition,revision FROM variant_templates ORDER BY id").fetch_all(&p).await {Ok(rows)=>Json(rows.into_iter().map(|(id,definition,revision)|json!({"id":id,"definition":definition,"revision":revision})).collect::<Vec<_>>()).into_response(),Err(_)=>fail(StatusCode::SERVICE_UNAVAILABLE,"Database unavailable")}
}
#[utoipa::path(put,path="/api/admin/variant-templates/{id}",params(("id"=Uuid,Path)),request_body=VariantTemplate,responses((status=200)),tag="admin catalog")]
pub async fn save_template(
    State(s): State<AppState>,
    a: AuthenticatedStaff,
    Path(id): Path<Uuid>,
    Json(input): Json<VariantTemplate>,
) -> Response {
    if let Err(e) = require_capability(&a, "catalog.write") {
        return e.into_response();
    }
    if id != input.id || !valid_option(&input.definition) {
        return fail(StatusCode::UNPROCESSABLE_ENTITY, "Invalid template");
    }
    let Some(p) = s.database else {
        return fail(StatusCode::SERVICE_UNAVAILABLE, "Database unavailable");
    };
    let Ok(mut tx) = p.begin().await else {
        return fail(StatusCode::SERVICE_UNAVAILABLE, "Database unavailable");
    };
    if sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(format!("variant-template:{id}"))
        .execute(&mut *tx)
        .await
        .is_err()
    {
        return fail(StatusCode::SERVICE_UNAVAILABLE, "Could not lock template");
    }
    let deleted = sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM audit_log WHERE action='product.variant_template_delete' AND entity_type='variant_template' AND entity_id=$1)").bind(id.to_string()).fetch_one(&mut *tx).await;
    match deleted {
        Ok(false) => {}
        Ok(true) => {
            return fail(
                StatusCode::CONFLICT,
                "This template was deleted. Reload before saving.",
            );
        }
        Err(_) => return fail(StatusCode::SERVICE_UNAVAILABLE, "Could not load template"),
    }
    match sqlx::query("INSERT INTO variant_templates(id,definition) VALUES($1,$2) ON CONFLICT(id) DO UPDATE SET definition=excluded.definition,revision=variant_templates.revision+1 WHERE variant_templates.revision=$3").bind(id).bind(json!(input.definition)).bind(input.revision).execute(&mut *tx).await {
      Ok(r) if r.rows_affected()==1=>{}, Ok(_)=>return fail(StatusCode::CONFLICT,"Template changed; reload before saving"), Err(_)=>return fail(StatusCode::SERVICE_UNAVAILABLE,"Could not save template")
    }
    if sqlx::query("INSERT INTO audit_log(actor_staff_user_id,action,entity_type,entity_id) VALUES($1,'product.variant_template','variant_template',$2)").bind(a.id).bind(id.to_string()).execute(&mut *tx).await.is_err() || tx.commit().await.is_err(){return fail(StatusCode::SERVICE_UNAVAILABLE,"Could not save template")}
    Json(json!({"ok":true})).into_response()
}
#[utoipa::path(put,path="/api/admin/products/{id}/variant-matrix",params(("id"=Uuid,Path)),request_body=SaveMatrix,responses((status=200)),tag="admin catalog")]
pub async fn save_matrix(
    State(s): State<AppState>,
    a: AuthenticatedStaff,
    Path(id): Path<Uuid>,
    Json(input): Json<SaveMatrix>,
) -> Response {
    if let Err(e) = require_capability(&a, "catalog.write") {
        return e.into_response();
    }
    let Some(ids) = validate_matrix(&input) else {
        return fail(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Invalid options or combinations (maximum 100 active variants)",
        );
    };
    let Some(p) = s.database else {
        return fail(StatusCode::SERVICE_UNAVAILABLE, "Database unavailable");
    };
    let Ok(mut tx) = p.begin().await else {
        return fail(StatusCode::SERVICE_UNAVAILABLE, "Database unavailable");
    };
    let Ok(Some(revision)) = sqlx::query_scalar::<_, i64>(
        "SELECT variant_revision FROM products WHERE id=$1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    else {
        return fail(StatusCode::NOT_FOUND, "Product not found");
    };
    if revision != input.revision {
        return fail(
            StatusCode::CONFLICT,
            "Product variants changed; reload before saving",
        );
    }
    for o in &input.config.options {
        for v in &o.values {
            for media_id in &v.media_ids {
                let owned=sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM product_media pm JOIN media_assets m ON m.id=pm.media_asset_id WHERE pm.product_id=$1 AND pm.media_asset_id=$2 AND m.status='ready')").bind(id).bind(media_id).fetch_one(&mut *tx).await.unwrap_or(false);
                if !owned {
                    return fail(
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "Photos must belong to this product",
                    );
                }
            }
        }
    }
    let Ok(existing) = sqlx::query_as::<_, (Uuid, Option<Value>)>(
        "SELECT id,selections FROM product_variants WHERE product_id=$1 FOR UPDATE",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await
    else {
        return fail(StatusCode::SERVICE_UNAVAILABLE, "Could not load variants");
    };
    for v in &input.variants {
        let selections = json!(v.selections);
        let old =
            v.id.and_then(|id| existing.iter().find(|(key, _)| *key == id));
        if v.id.is_some() && old.is_none() {
            return fail(StatusCode::UNPROCESSABLE_ENTITY, "Unknown variant");
        }
        if old.is_some_and(|(_, s)| s.as_ref().is_some_and(|s| s != &selections)) {
            return fail(
                StatusCode::CONFLICT,
                "An existing variant's options cannot change; create a new combination",
            );
        }
        let title = input
            .config
            .options
            .iter()
            .filter_map(|o| {
                o.values
                    .iter()
                    .find(|x| Some(&x.code) == v.selections.get(&o.code))
                    .map(|v| v.label.clone())
            })
            .collect::<Vec<_>>()
            .join(" · ");
        let labels = input
            .config
            .options
            .iter()
            .filter_map(|o| {
                o.values
                    .iter()
                    .find(|x| Some(&x.code) == v.selections.get(&o.code))
                    .map(|v| (o.label.clone(), v.label.clone()))
            })
            .collect::<BTreeMap<_, _>>();
        let variant_id = v.id.unwrap_or_else(Uuid::now_v7);
        let result = if old.is_some() {
            sqlx::query("UPDATE product_variants SET selections=$2,title=CASE WHEN $3='' THEN title ELSE $3 END,option_values=$4,price_minor=$5,retired=$6 WHERE id=$1").bind(variant_id).bind(if v.retired && old.is_some_and(|(_,s)|s.is_none()) && v.selections.is_empty(){None}else{Some(&selections)}).bind(&title).bind(json!(labels)).bind(v.price_minor).bind(v.retired).execute(&mut *tx).await
        } else {
            sqlx::query("INSERT INTO product_variants(id,product_id,title,sku,price_minor,currency,option_values,selections,retired,position) VALUES($1,$2,$3,'generated',$4,'EUR',$5,$6,$7,(SELECT count(*) FROM product_variants WHERE product_id=$2))").bind(variant_id).bind(id).bind(if title.is_empty(){"Default"}else{&title}).bind(v.price_minor).bind(json!(labels)).bind(selections).bind(v.retired).execute(&mut *tx).await
        };
        if result.is_err() {
            return fail(
                StatusCode::CONFLICT,
                "Duplicate combination or immutable variant identity",
            );
        }
        let Ok(old_quantity) = sqlx::query_scalar::<_, i64>(
            "SELECT available_quantity FROM inventory_items WHERE variant_id=$1 FOR UPDATE",
        )
        .bind(variant_id)
        .fetch_one(&mut *tx)
        .await
        else {
            return fail(StatusCode::SERVICE_UNAVAILABLE, "Inventory unavailable");
        };
        if old.is_some() && v.expected_quantity != Some(old_quantity) {
            return fail(
                StatusCode::CONFLICT,
                "Inventory changed; reload before saving",
            );
        }
        if old_quantity != v.available_quantity {
            let Some(delta) = v.available_quantity.checked_sub(old_quantity) else {
                return fail(StatusCode::UNPROCESSABLE_ENTITY, "Invalid stock");
            };
            if sqlx::query("UPDATE inventory_items SET available_quantity=$2 WHERE variant_id=$1").bind(variant_id).bind(v.available_quantity).execute(&mut *tx).await.is_err() || sqlx::query("INSERT INTO inventory_movements(id,variant_id,actor_staff_user_id,movement_type,quantity_delta,resulting_available_quantity,reason) VALUES($1,$2,$3,'adjustment',$4,$5,'Variant matrix stock adjustment')").bind(Uuid::now_v7()).bind(variant_id).bind(a.id).bind(delta).bind(v.available_quantity).execute(&mut *tx).await.is_err(){return fail(StatusCode::SERVICE_UNAVAILABLE,"Could not save inventory")}
        }
    }
    for (variant_id, _) in existing {
        if !ids.contains(&variant_id)
            && sqlx::query("UPDATE product_variants SET retired=true WHERE id=$1")
                .bind(variant_id)
                .execute(&mut *tx)
                .await
                .is_err()
        {
            return fail(StatusCode::SERVICE_UNAVAILABLE, "Could not retire variant");
        }
    }
    if sqlx::query("UPDATE products SET variant_config=$2,variant_revision=variant_revision+1 WHERE id=$1").bind(id).bind(json!(input.config)).execute(&mut *tx).await.is_err() || sqlx::query("INSERT INTO audit_log(actor_staff_user_id,action,entity_type,entity_id) VALUES($1,'product.variant_matrix','product',$2)").bind(a.id).bind(id.to_string()).execute(&mut *tx).await.is_err() || tx.commit().await.is_err(){return fail(StatusCode::SERVICE_UNAVAILABLE,"Could not save matrix")}
    Json(json!({"ok":true})).into_response()
}

pub fn media_for(config: &Value, selections: Option<&Value>) -> Vec<Uuid> {
    if config.get("color_photos").and_then(Value::as_bool) != Some(true) {
        return vec![];
    }
    let Ok(c) = serde_json::from_value::<VariantConfig>(config.clone()) else {
        return vec![];
    };
    c.options
        .iter()
        .filter(|o| o.control == "swatch")
        .filter_map(|o| {
            o.values.iter().find(|v| {
                selections
                    .and_then(|s| s.get(&o.code))
                    .and_then(Value::as_str)
                    == Some(v.code.as_str())
            })
        })
        .flat_map(|v| v.media_ids.clone())
        .collect()
}

fn validate_matrix(input: &SaveMatrix) -> Option<BTreeSet<Uuid>> {
    let mut codes = BTreeSet::new();
    let mut combinations = BTreeSet::new();
    let mut labels = BTreeSet::new();
    let mut ids = BTreeSet::new();
    if input
        .config
        .options
        .iter()
        .filter(|o| o.control == "swatch")
        .count()
        > 1
        || input.config.options.len() > 6
        || !input.config.options.iter().all(|o| {
            valid_option(o) && codes.insert(&o.code) && labels.insert(o.label.trim().to_lowercase())
        })
        || input.variants.is_empty()
        || input.variants.iter().filter(|v| !v.retired).count() > 100
        || !input.variants.iter().all(|v| {
            v.price_minor >= 0
                && (!v.retired || v.id.is_some())
                && v.selections.len() <= 6
                && v.selections
                    .iter()
                    .all(|(k, v)| valid_code(k) && valid_code(v))
                && v.id.is_none_or(|id| ids.insert(id))
                && (v.retired && v.selections.is_empty()
                    || combinations.insert(json!(v.selections).to_string()))
                && (v.retired
                    || (v.selections.len() == input.config.options.len()
                        && input.config.options.iter().all(|o| {
                            v.selections
                                .get(&o.code)
                                .is_some_and(|c| o.values.iter().any(|v| &v.code == c))
                        })))
        })
    {
        return None;
    }
    Some(ids)
}

#[derive(Serialize, ToSchema)]
pub struct MatrixPreview {
    pub active_count: usize,
    pub combination_count: usize,
}
#[utoipa::path(post,path="/api/admin/products/{id}/variant-matrix/preview",params(("id"=Uuid,Path)),request_body=SaveMatrix,responses((status=200,body=MatrixPreview)),tag="admin catalog")]
pub async fn preview_matrix(
    State(s): State<AppState>,
    a: AuthenticatedStaff,
    Path(id): Path<Uuid>,
    Json(input): Json<SaveMatrix>,
) -> Response {
    if let Err(e) = require_capability(&a, "catalog.write") {
        return e.into_response();
    }
    if validate_matrix(&input).is_none() {
        return fail(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Invalid options or combinations (maximum 100 active variants)",
        );
    }
    let Some(p) = s.database else {
        return fail(StatusCode::SERVICE_UNAVAILABLE, "Database unavailable");
    };
    match sqlx::query_scalar::<_, i64>("SELECT variant_revision FROM products WHERE id=$1")
        .bind(id)
        .fetch_optional(&p)
        .await
    {
        Ok(Some(revision)) if revision == input.revision => Json(MatrixPreview {
            active_count: input.variants.iter().filter(|v| !v.retired).count(),
            combination_count: input.variants.len(),
        })
        .into_response(),
        Ok(Some(_)) => fail(
            StatusCode::CONFLICT,
            "Product variants changed; reload before saving",
        ),
        Ok(None) => fail(StatusCode::NOT_FOUND, "Product not found"),
        Err(_) => fail(StatusCode::SERVICE_UNAVAILABLE, "Database unavailable"),
    }
}

#[derive(Deserialize, ToSchema)]
pub struct DeleteTemplateRequest {
    pub revision: i64,
}

#[utoipa::path(delete,path="/api/admin/variant-templates/{id}",params(("id"=Uuid,Path)),request_body=DeleteTemplateRequest,responses((status=200),(status=409)),tag="admin catalog")]
pub async fn delete_template(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(id): Path<Uuid>,
    Json(input): Json<DeleteTemplateRequest>,
) -> Response {
    if let Err(e) = require_capability(&actor, "catalog.write") {
        return e.into_response();
    }
    let Some(pool) = state.database else {
        return fail(StatusCode::SERVICE_UNAVAILABLE, "Database unavailable");
    };
    let Ok(mut tx) = pool.begin().await else {
        return fail(StatusCode::SERVICE_UNAVAILABLE, "Database unavailable");
    };
    if sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(format!("variant-template:{id}"))
        .execute(&mut *tx)
        .await
        .is_err()
    {
        return fail(StatusCode::SERVICE_UNAVAILABLE, "Could not lock template");
    }
    match sqlx::query("DELETE FROM variant_templates WHERE id=$1 AND revision=$2")
        .bind(id)
        .bind(input.revision)
        .execute(&mut *tx)
        .await
    {
        Ok(result) if result.rows_affected() == 1 => {}
        Ok(_) => {
            return fail(
                StatusCode::CONFLICT,
                "This template changed or was removed. Reload before deleting.",
            );
        }
        Err(_) => return fail(StatusCode::SERVICE_UNAVAILABLE, "Could not delete template"),
    }
    if sqlx::query("INSERT INTO audit_log(actor_staff_user_id,action,entity_type,entity_id) VALUES($1,'product.variant_template_delete','variant_template',$2)")
        .bind(actor.id).bind(id.to_string()).execute(&mut *tx).await.is_err() || tx.commit().await.is_err() {
        return fail(StatusCode::SERVICE_UNAVAILABLE, "Could not delete template");
    }
    Json(json!({"ok":true})).into_response()
}
