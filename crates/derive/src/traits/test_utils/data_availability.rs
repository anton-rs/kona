

// A mock data iterator for testing.
// #[derive(Debug, Clone, Default)]
// pub struct TestDataIter<T: Into<Bytes>> {
//     /// The data to iterate over.
//     pub data: Vec<T>,
// }

// impl<T: Into<Bytes>> DataIter<T> for TestDataIter<T> {
//     fn next(&mut self) -> StageResult<T> {
//         if let Some(data) = self.data.pop() {
//             Ok(data)
//         } else {
//             Err(crate::types::StageError::Eof)
//         }
//     }
// }
//
// /// A mock data availability provider for testing.
// #[derive(Debug, Clone, Default)]
// pub struct TestDataAvailabilityProvider<T: Into<Bytes>> {
//     /// Maps block hashes to data iterators using a tuple list.
//     pub data: Vec<(B256, TestDataIter<T>)>,
// }
//
// impl<T: Into<Bytes>> TestDataAvailabilityProvider<T> {
//     pub fn insert_data(&mut self, hash: B256, data: TestDataIter<T>) {
//         self.data.push((hash, data));
//     }
// }
//
// #[async_trait]
// impl<B: Into<Bytes>> DataAvailabilityProvider for TestDataAvailabilityProvider<B> {
//     type DataIter<T> = TestDataIter<T>;
//
//     async fn open_data(
//         &self,
//         block_ref: &BlockInfo,
//         _batcher_address: alloy_primitives::Address,
//     ) -> Result<Self::DataIter<B>> {
//         if let Some((_, data)) = self.data.iter().find(|(h, _)| *h == block_ref.hash) {
//             let res = data.clone();
//             Ok(res)
//         } else {
//             Err(anyhow::anyhow!("Data not found"))
//         }
//     }
// }
