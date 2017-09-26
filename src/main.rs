// Copyright 2017 The Exonum Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Import crates with necessary types into a new project.

extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate exonum;
extern crate router;
extern crate bodyparser;
extern crate iron;

// Import necessary types from crates.

use exonum::blockchain::{self, Blockchain, Service, GenesisConfig, ValidatorKeys, Transaction,
                         ApiContext};
use exonum::node::{Node, NodeConfig, NodeApiConfig, TransactionSend, ApiSender, NodeChannel};
use exonum::messages::{RawTransaction, FromRaw};
use exonum::storage::{Fork, MemoryDB, Entry};
use exonum::crypto::{Hash};
use exonum::encoding::{self, Field};
use exonum::api::{Api, ApiError};
use iron::prelude::*;
use iron::Handler;
use router::Router;
use serde_json::value::Value;

// // // // // // // // // // CONSTANTS // // // // // // // // // //

// Define service ID for the service trait.

const SERVICE_ID: u16 = 1;

// Define constants for transaction types within the service.

const TX_GRANT: u16 = 1;
const TX_DENY: u16 = 2;

// // // // // // // // // // PERSISTENT DATA // // // // // // // // // //

// Declare the data to be stored in the blockchain. In the present case,
// declare a type for storing information about the wallet and its balance.

// Declare a [serializable][1]
// [1]: https://github.com/exonum/exonum-doc/blob/master/src/architecture/serialization.md
// struct and determine bounds of its fields with `encoding_struct!` macro.
encoding_struct! {
    struct MatrixEntry {
        const SIZE = 8;

        field subject:      u32     [00 => 04]
        field permission:   u32     [04 => 08]
    }
}

encoding_struct! {
    struct Matrix {
        const SIZE = 8;

        field values:   Vec<MatrixEntry>  [00 => 08]
    }
}

impl Matrix {
    pub fn grant(&mut self, subject: u32, permission: u32) {
        let values = &mut self.values();

        // slices are of the same length by design
        let len = values.len();

        for i in 0..len {
            let s = values[i].subject();
            let p = values[i].permission();

            if s==subject && p==permission {
                // such grant already exists
                return;
            }
        }

        let entry = MatrixEntry::new(subject, permission);
        &values.push(entry);

        Field::write(values, &mut self.raw, 0, 8);
    }

    pub fn deny(&mut self, subject: u32, permission: u32) {
        let values = &mut self.values();

        // slices are of the same length by design
        let len = values.len();
        let mut remove_index = len;

        for i in 0..len {
            let s = values[i].subject();
            let p = values[i].permission();

            if s==subject && p==permission {
                // this grant has to be removed
                remove_index = i;
                break;
            }
        }

        if remove_index==len {
            return;
        }

        values.swap_remove(remove_index);

        Field::write(values, &mut self.raw, 00, 08);
    }
}

// // // // // // // // // // DATA LAYOUT // // // // // // // // // //

/// Create schema of the key-value storage implemented by `MemoryDB`. In the
/// present case a `Fork` of the database is used.
pub struct MatrixSchema<'a> {
    view: &'a mut Fork,
}

/// Declare layout of the data. Use an instance of [`MapIndex`][2]
/// [2]: https://github.com/exonum/exonum-doc/blob/master/src/architecture/storage.md#mapindex
/// to keep wallets in storage. Index values are serialized `Wallet` structs.
///
/// Isolate the wallets map into a separate entity by adding a unique prefix,
/// i.e. the first argument to the `MapIndex::new` call.
impl<'a> MatrixSchema<'a> {
    // pub fn access_control(&mut self) -> ListIndex<&mut Fork, Matrix> {
    //     let prefix = blockchain::gen_prefix(SERVICE_ID, 0, &());
    //     ListIndex::new(prefix, self.view)
    // }
    pub fn access_control(&mut self) -> Entry<&mut Fork, Matrix> {
        let prefix = blockchain::gen_prefix(SERVICE_ID, 0, &());
        Entry::new(prefix, self.view)
    }
}

// // // // // // // // // // TRANSACTIONS // // // // // // // // // //
message! {
    struct TxGrant {
        const TYPE = SERVICE_ID;
        const ID = TX_GRANT;
        const SIZE = 8;

        field entry:    MatrixEntry   [00=>08]
    }
}

message! {
    struct TxDeny {
        const TYPE = SERVICE_ID;
        const ID = TX_DENY;
        const SIZE = 16;

        field entry:    MatrixEntry   [00=>08]
    }
}
// // // // // // // // // // CONTRACTS // // // // // // // // // //

/// Execute a transaction.
impl Transaction for TxGrant {
    /// Verify integrity of the transaction by checking the transaction
    /// signature.
    fn verify(&self) -> bool {
        true
    }

    /// Apply logic to the storage when executing the transaction.
    fn execute(&self, view: &mut Fork) {
        let mut schema = MatrixSchema { view };

        if let Some(mut matrix) = schema.access_control().get() {
            let e = self.entry();
            let s = e.subject();
            let p = e.permission();
            println!("Granting {} with {}", s, p);

            matrix.grant(s, p);
            println!("After Grant: {:?}\n", matrix);
        }
    }
}

impl Transaction for TxDeny {
    /// Verify integrity of the transaction by checking the transaction
    /// signature.
    fn verify(&self) -> bool {
        true
    }

    /// Apply logic to the storage when executing the transaction.
    fn execute(&self, view: &mut Fork) {
        let mut schema = MatrixSchema { view };

        if let Some(mut matrix) = schema.access_control().get() {
            let e = self.entry();
            let s = e.subject();
            let p = e.permission();
            println!("Denying {} with {}", s, p);

            matrix.deny(s, p);
            println!("After Deny: {:?}\n", matrix);
        }

        // let mut matrix = schema.access_control();
        // let mut m = matrix
        //             .pop()
        //             .unwrap_or(Matrix::new(vec![]));

        // let e = self.entry();
        // let s = e.subject();
        // let p = e.permission();
        // println!("Denying {} with {}", s, p);

        // m.deny(s, p);
        // println!("Deny: {:?}\n", m);

        // matrix.push(m);
    }
}
// // // // // // // // // // REST API // // // // // // // // // //

/// Implement the node API.
#[derive(Clone)]
struct ACApi {
    channel: ApiSender<NodeChannel>,
    blockchain: Blockchain,
}

/// Shortcut to get data on wallets.
impl ACApi {
    fn get_access_control(&self) -> Option<Matrix> {
        let mut view = self.blockchain.fork();
        let mut schema = MatrixSchema { view: &mut view };
        schema.access_control().get()
    }
}

/// Add an enum which joins transactions of both types to simplify request
/// processing.
#[serde(untagged)]
#[derive(Clone, Serialize, Deserialize)]
enum TransactionRequest {
    Grant(TxGrant),
    Deny(TxDeny),
}

/// Implement a trait for the enum for deserialized `TransactionRequest`s
/// to fit into the node channel.
impl Into<Box<Transaction>> for TransactionRequest {
    fn into(self) -> Box<Transaction> {
        match self {
            TransactionRequest::Grant(trans) => Box::new(trans),
            TransactionRequest::Deny(trans) => Box::new(trans),
        }
    }
}

/// The structure returned by the REST API.
#[derive(Serialize, Deserialize)]
struct TransactionResponse {
    tx_hash: Hash,
}

/// Implement the `Api` trait.
/// `Api` facilitates conversion between transactions/read requests and REST
/// endpoints; for example, it parses `POSTed` JSON into the binary transaction
/// representation used in Exonum internally.
impl Api for ACApi {
    fn wire(&self, router: &mut Router) {

        let self_ = self.clone();
        let transaction = move |req: &mut Request| -> IronResult<Response> {
            match req.get::<bodyparser::Struct<TransactionRequest>>() {
                Ok(Some(transaction)) => {
                    let transaction: Box<Transaction> = transaction.into();
                    let tx_hash = transaction.hash();
                    self_.channel.send(transaction).map_err(ApiError::Events)?;
                    let json = TransactionResponse { tx_hash };
                    self_.ok_response(&serde_json::to_value(&json).unwrap())
                }
                Ok(None) => Err(ApiError::IncorrectRequest("Empty request body".into()))?,
                Err(e) => Err(ApiError::IncorrectRequest(Box::new(e)))?,
            }
        };

        // Gets status of ac.
        let self_ = self.clone();
        let ac_info = move |_: &mut Request| -> IronResult<Response> {
            if let Some(ac) = self_.get_access_control() {
                self_.ok_response(&serde_json::to_value(ac).unwrap())
            } else {
                self_.not_found_response(
                    &serde_json::to_value("no ac")
                        .unwrap(),
                )
            }
        };

        // Bind the transaction handler to a specific route.
        router.post("/v1/ac/transaction", transaction, "transaction");
        router.get("/v1/ac", ac_info, "ac_info");
    }
}

// // // // // // // // // // SERVICE DECLARATION // // // // // // // // // //

/// Define the service.
struct ACService;

/// Implement a `Service` trait for the service.
impl Service for ACService {
    fn service_name(&self) -> &'static str {
        "cryptocurrency"
    }

    fn service_id(&self) -> u16 {
        SERVICE_ID
    }

    /// Implement a method to deserialize transactions coming to the node.
    fn tx_from_raw(&self, raw: RawTransaction) -> Result<Box<Transaction>, encoding::Error> {
        let trans: Box<Transaction> = match raw.message_type() {
            TX_GRANT => Box::new(TxGrant::from_raw(raw)?),
            TX_DENY => Box::new(TxDeny::from_raw(raw)?),
            _ => {
                return Err(encoding::Error::IncorrectMessageType {
                    message_type: raw.message_type(),
                });
            }
        };
        Ok(trans)
    }

    /// Create a REST `Handler` to process web requests to the node.
    fn public_api_handler(&self, ctx: &ApiContext) -> Option<Box<Handler>> {
        let mut router = Router::new();
        let api = ACApi {
            channel: ctx.node_channel().clone(),
            blockchain: ctx.blockchain().clone(),
        };
        api.wire(&mut router);
        Some(Box::new(router))
    }

    fn initialize(&self, fork: &mut Fork) -> Value {
        // let mut handler = self.handler.lock().unwrap();
        // let cfg = self.genesis.clone();
        // let (_, addr) = cfg.redeem_script();
        // if handler.client.is_some() {
        //     handler.import_address(&addr).unwrap();
        // }
        // AnchoringSchema::new(fork).create_genesis_config(&cfg);
        // serde_json::to_value(cfg).unwrap()


        // MatrixSchema::new(fork);

        let mut schema = MatrixSchema { view: fork };
        let matrix = Matrix::new(vec![]);
        schema.view.
        // schema.access_control().set(matrix);

        serde_json::to_value().unwrap()
    }
}

// // // // // // // // // // ENTRY POINT // // // // // // // // // //

fn main() {
    exonum::helpers::init_logger().unwrap();

    println!("Creating in-memory database...");
    let db = MemoryDB::new();
    let services: Vec<Box<Service>> = vec![Box::new(ACService)];
    let blockchain = Blockchain::new(Box::new(db), services);

    let (consensus_public_key, consensus_secret_key) = exonum::crypto::gen_keypair();
    let (service_public_key, service_secret_key) = exonum::crypto::gen_keypair();

    let validator_keys = ValidatorKeys {
        consensus_key: consensus_public_key,
        service_key: service_public_key,
    };
    let genesis = GenesisConfig::new(vec![validator_keys].into_iter());

    let api_address = "0.0.0.0:8000".parse().unwrap();
    let api_cfg = NodeApiConfig {
        public_api_address: Some(api_address),
        ..Default::default()
    };

    let peer_address = "0.0.0.0:2000".parse().unwrap();

    let node_cfg = NodeConfig {
        listen_address: peer_address,
        peers: vec![],
        service_public_key,
        service_secret_key,
        consensus_public_key,
        consensus_secret_key,
        genesis,
        external_address: None,
        network: Default::default(),
        whitelist: Default::default(),
        api: api_cfg,
        mempool: Default::default(),
        services_configs: Default::default(),
    };

    println!("Starting a single node...");
    let mut node = Node::new(blockchain, node_cfg);

    println!("Blockchain is ready for transactions!");
    node.run().unwrap();


}
