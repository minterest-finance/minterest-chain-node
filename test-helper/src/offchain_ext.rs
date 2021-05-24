//! TODO DESCRIPTION

use frame_support::sp_runtime::app_crypto::sp_core::offchain::{
	Externalities, HttpError, HttpRequestId, HttpRequestStatus, OpaqueNetworkState, StorageKind, Timestamp,
};

use frame_support::sp_runtime::app_crypto::sp_core::OpaquePeerId;

use std::time::{SystemTime, UNIX_EPOCH};

// It can be done like more general by taking a struct suitable hooks.
/// Allows you to hook into timestamp call
pub struct OffChainExtWithHooks<T> {
	inner: Box<T>,
	timestamp_hook: Option<Box<dyn Fn(Timestamp, &mut dyn Externalities) -> Timestamp + Send>>,
}

impl<T> OffChainExtWithHooks<T>
where
	T: Externalities,
{
	pub fn new(
		inner: T,
		timestamp_hook: Option<Box<dyn Fn(Timestamp, &mut dyn Externalities) -> Timestamp + Send>>,
	) -> Self {
		Self {
			inner: Box::new(inner),
			timestamp_hook,
		}
	}
}

impl<T> Externalities for OffChainExtWithHooks<T>
where
	T: Externalities,
{
	fn is_validator(&self) -> bool {
		self.inner.is_validator()
	}

	fn network_state(&self) -> Result<OpaqueNetworkState, ()> {
		self.inner.network_state()
	}

	/// Returns timestamp of hook_fn. Otherwise returns system timestamp
	fn timestamp(&mut self) -> Timestamp {
		let inner_timestamp = self.inner.timestamp();
		return if let Some(ref mut timestamp_hook_fn) = self.timestamp_hook {
			timestamp_hook_fn(inner_timestamp, self.inner.as_mut())
		} else {
			let start = SystemTime::now();
			let since_the_epoch = start.duration_since(UNIX_EPOCH).unwrap();
			Timestamp::from_unix_millis(since_the_epoch.as_millis() as u64)
		};
	}

	fn sleep_until(&mut self, deadline: Timestamp) {
		self.inner.sleep_until(deadline)
	}

	fn random_seed(&mut self) -> [u8; 32] {
		self.inner.random_seed()
	}

	fn local_storage_set(&mut self, kind: StorageKind, key: &[u8], value: &[u8]) {
		self.inner.local_storage_set(kind, key, value)
	}

	fn local_storage_clear(&mut self, kind: StorageKind, key: &[u8]) {
		self.inner.local_storage_clear(kind, key)
	}

	fn local_storage_compare_and_set(
		&mut self,
		kind: StorageKind,
		key: &[u8],
		old_value: Option<&[u8]>,
		new_value: &[u8],
	) -> bool {
		self.inner
			.local_storage_compare_and_set(kind, key, old_value, new_value)
	}

	fn local_storage_get(&mut self, kind: StorageKind, key: &[u8]) -> Option<Vec<u8>> {
		self.inner.local_storage_get(kind, key)
	}

	fn http_request_start(&mut self, method: &str, uri: &str, meta: &[u8]) -> Result<HttpRequestId, ()> {
		self.inner.http_request_start(method, uri, meta)
	}

	fn http_request_add_header(&mut self, request_id: HttpRequestId, name: &str, value: &str) -> Result<(), ()> {
		self.inner.http_request_add_header(request_id, name, value)
	}

	fn http_request_write_body(
		&mut self,
		request_id: HttpRequestId,
		chunk: &[u8],
		deadline: Option<Timestamp>,
	) -> Result<(), HttpError> {
		self.inner.http_request_write_body(request_id, chunk, deadline)
	}

	fn http_response_wait(&mut self, ids: &[HttpRequestId], deadline: Option<Timestamp>) -> Vec<HttpRequestStatus> {
		self.inner.http_response_wait(ids, deadline)
	}

	fn http_response_headers(&mut self, request_id: HttpRequestId) -> Vec<(Vec<u8>, Vec<u8>)> {
		self.inner.http_response_headers(request_id)
	}

	fn http_response_read_body(
		&mut self,
		request_id: HttpRequestId,
		buffer: &mut [u8],
		deadline: Option<Timestamp>,
	) -> Result<usize, HttpError> {
		self.inner.http_response_read_body(request_id, buffer, deadline)
	}

	fn set_authorized_nodes(&mut self, nodes: Vec<OpaquePeerId>, authorized_only: bool) {
		self.inner.set_authorized_nodes(nodes, authorized_only)
	}
}
