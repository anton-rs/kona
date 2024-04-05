


func DeriveDeposits(receipts []*types.Receipt, depositContractAddr common.Address) ([]hexutil.Bytes, error) {
	var result error
	userDeposits, err := UserDeposits(receipts, depositContractAddr)
	if err != nil {
		result = multierror.Append(result, err)
	}
	encodedTxs := make([]hexutil.Bytes, 0, len(userDeposits))
	for i, tx := range userDeposits {
		opaqueTx, err := types.NewTx(tx).MarshalBinary()
		if err != nil {
			result = multierror.Append(result, fmt.Errorf("failed to encode user tx %d", i))
		} else {
			encodedTxs = append(encodedTxs, opaqueTx)
		}
	}
	return encodedTxs, result
}
