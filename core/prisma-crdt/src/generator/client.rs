use crate::generator::prelude::*;

fn create_operation_fn() -> TokenStream {
	quote! {
		pub async fn _create_operation(&self, typ: ::prisma_crdt::CRDTOperationType) {
			let timestamp = ::uhlc::NTP64(0); // TODO: actual timestamps

			let timestamp_bytes = vec![0];

			match &typ {
				::prisma_crdt::CRDTOperationType::Shared(::prisma_crdt::SharedOperation {
					record_id,
					model,
					data,
				}) => {
					let (kind, data) = match data {
						::prisma_crdt::SharedOperationData::Create(typ) => {
							("c".to_string(), ::serde_json::to_vec(typ).unwrap())
						}
						::prisma_crdt::SharedOperationData::Update { field, value } => {
							("u".to_string() + field, ::serde_json::to_vec(value).unwrap())
						}
						::prisma_crdt::SharedOperationData::Delete => ("d".to_string(), vec![]),
					};

					self.client
						.shared_operation()
						.create(
							timestamp_bytes,
							::serde_json::to_vec(&record_id).unwrap(),
							kind,
							model.to_string(),
							data,
							crate::prisma::node::local_id::equals(self.node_local_id),
							vec![],
						)
						.exec()
						.await;
				}
				::prisma_crdt::CRDTOperationType::Owned(op) => {
					self.client
						.owned_operation()
						.create(
							timestamp_bytes,
							::serde_json::to_vec(op).unwrap(),
							crate::prisma::node::local_id::equals(self.node_local_id),
							vec![],
						)
						.exec()
						.await;
				}
				::prisma_crdt::CRDTOperationType::Relation(::prisma_crdt::RelationOperation {
					relation,
					relation_item,
					relation_group,
					data,
				}) => {
					let (kind, data) = match data {
						::prisma_crdt::RelationOperationData::Create => ("c".to_string(), vec![]),
						::prisma_crdt::RelationOperationData::Update { field, value } => {
							("u".to_string() + field, ::serde_json::to_vec(value).unwrap())
						}
						::prisma_crdt::RelationOperationData::Delete => ("d".to_string(), vec![]),
					};

					self.client
						.relation_operation()
						.create(
							timestamp_bytes,
							relation.to_string(),
							relation_item.clone(),
							relation_group.clone(),
							kind,
							data,
							crate::prisma::node::local_id::equals(self.node_local_id),
							vec![],
						)
						.exec()
						.await;
				}
			}

			let op = ::prisma_crdt::CRDTOperation::new(self.node_id.clone(), timestamp, typ);

			self.operation_sender.send(op).await;
		}
	}
}

fn actions_accessors<'a>(datamodel: &'a Datamodel<'a>) -> impl Iterator<Item = TokenStream> + 'a {
	datamodel.models.iter().map(|model| {
		let Model {
			name_snake, typ, ..
		} = model.as_ref();

		match typ {
			ModelType::Local { .. } => quote! {
				pub fn #name_snake(&self) -> crate::prisma::#name_snake::Actions {
					self.client.#name_snake()
				}
			},
			_ => quote! {
				pub fn #name_snake(&self) -> super::#name_snake::Actions {
					super::#name_snake::Actions::new(self)
				}
			},
		}
	})
}

pub fn generate<'a>(datamodel: &'a Datamodel<'a>) -> TokenStream {
	let create_operation_fn = create_operation_fn();

	let actions_accessors = actions_accessors(&datamodel);

	quote! {
		mod _prisma {
			pub struct PrismaCRDTClient {
				pub(super) client: crate::prisma::PrismaClient,
				pub node_id: Vec<u8>,
				pub node_local_id: i32,
				operation_sender: ::tokio::sync::mpsc::Sender<::prisma_crdt::CRDTOperation>,
			}

			impl PrismaCRDTClient {
				pub(super) fn _new(
					client: crate::prisma::PrismaClient,
					(node_id, node_local_id): (Vec<u8>, i32),
					operation_sender: ::tokio::sync::mpsc::Sender<::prisma_crdt::CRDTOperation>,
				) -> Self {
					Self {
						client,
						operation_sender,
						node_id,
						node_local_id,
					}
				}

				#create_operation_fn

				#(#actions_accessors)*
			}
		}
	}
}
