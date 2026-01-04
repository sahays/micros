use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClientType {
    Web,
    Service,
    Mobile,
}

impl fmt::Display for ClientType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ClientType::Web => write!(f, "web"),
            ClientType::Service => write!(f, "service"),
            ClientType::Mobile => write!(f, "mobile"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Client {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    pub id: Option<ObjectId>,
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub client_id: String,
    #[schema(read_only)]
    pub client_secret_hash: String,
    #[schema(read_only)]
    pub previous_client_secret_hash: Option<String>,
    #[serde(
        default,
        with = "optional_chrono_datetime_as_bson_datetime",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = Option<String>, format = "date-time")]
    pub previous_secret_expiry: Option<chrono::DateTime<chrono::Utc>>,
    #[schema(example = "My BFF App")]
    pub app_name: String,
    pub app_type: ClientType,
    #[schema(example = "signing-secret-key")]
    pub signing_secret: String,
    #[schema(example = 100)]
    pub rate_limit_per_min: u32,
    pub allowed_origins: Vec<String>,
    pub enabled: bool,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    #[schema(value_type = String, format = "date-time")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    #[schema(value_type = String, format = "date-time")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub mod optional_chrono_datetime_as_bson_datetime {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(val: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match val {
            Some(date) => {
                mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime::serialize(
                    date, serializer,
                )
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wrapper(
            #[serde(with = "mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime")]
            DateTime<Utc>,
        );

        let wrapper = Option::<Wrapper>::deserialize(deserializer)?;
        Ok(wrapper.map(|w| w.0))
    }
}

impl Client {
    pub fn new(
        client_id: String,
        client_secret_hash: String,
        signing_secret: String,
        app_name: String,
        app_type: ClientType,
        rate_limit_per_min: u32,
        allowed_origins: Vec<String>,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: None,
            client_id,
            client_secret_hash,
            previous_client_secret_hash: None,
            previous_secret_expiry: None,
            app_name,
            app_type,
            signing_secret,
            rate_limit_per_min,
            allowed_origins,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }
}
