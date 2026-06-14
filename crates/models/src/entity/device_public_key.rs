use sea_orm::entity::prelude::*;

/// A device's public key, registered so the server can wrap offline book content
/// to it (E3 offline reading for users without `DownloadFile`). The private key
/// never leaves the device's Secure Enclave.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "device_public_keys")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
	pub id: String,
	#[sea_orm(column_type = "Text")]
	pub user_id: String,
	#[sea_orm(column_type = "Text")]
	pub device_id: String,
	/// The device's P-256 public key, X9.63 uncompressed (65 bytes).
	pub public_key: Vec<u8>,
	pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
