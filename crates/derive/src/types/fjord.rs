//! Module containing a [Transaction] builder for the Fjord network upgrade transactions.
//!
//! [Transaction]: alloy_consensus::Transaction

use crate::types::{
    upgrade_to_calldata, RawTransaction, UpgradeDepositSource, GAS_PRICE_ORACLE_ADDRESS,
};
use alloc::{string::String, vec, vec::Vec};
use alloy_primitives::{address, bytes, Address, Bytes, TxKind, U256};
use op_alloy_consensus::{Encodable2718, OpTxEnvelope, TxDeposit};
use spin::Lazy;

/// The L1 Info Depositer Address.
pub const L1_INFO_DEPOSITER_ADDRESS: Address = address!("deaddeaddeaddeaddeaddeaddeaddeaddead0001");

/// Fjord Gas Price Oracle Deployer Address.
pub const GAS_PRICE_ORACLE_FJORD_DEPLOYER_ADDRESS: Address =
    address!("4210000000000000000000000000000000000002");

/// Fjord Gas Price Oracle source.
static DEPLOY_FJORD_GAS_PRICE_ORACLE_SOURCE: Lazy<UpgradeDepositSource> = Lazy::new(|| {
    UpgradeDepositSource { intent: String::from("Fjord: Gas Price Oracle Deployment") }
});

/// [UpgradeDepositSource] for the source code update to the Fjord Gas Price Oracle.
static UPDATE_FJORD_GAS_PRICE_ORACLE_SOURCE: Lazy<UpgradeDepositSource> = Lazy::new(|| {
    UpgradeDepositSource { intent: String::from("Fjord: Gas Price Oracle Proxy Update") }
});

/// Fjord Gas Price Oracle address.
pub const FJORD_GAS_PRICE_ORACLE_ADDRESS: Address =
    address!("a919894851548179a0750865e7974da599c0fac7");

/// [UpgradeDepositSource] for setting the Fjord Gas Price Oracle.
static ENABLE_FJORD_SOURCE: Lazy<UpgradeDepositSource> = Lazy::new(|| UpgradeDepositSource {
    intent: String::from("Fjord: Gas Price Oracle Set Fjord"),
});

/// Input data for setting the Fjord Gas Price Oracle.
pub const ENABLE_FJORD_FUNC_SIGNATURE: &str = "setFjord()";

/// The Set Fjord Four Byte Method Signature.
pub const SET_FJORD_METHOD_SIGNATURE: &[u8] = &[0x8e, 0x98, 0xb1, 0x06];

/// Builder wrapper for the Fjord network updgrade.
#[derive(Debug, Default)]
pub struct FjordTransactionBuilder;

impl FjordTransactionBuilder {
    /// Constructs the Ecotone network upgrade transactions.
    pub fn build_txs() -> anyhow::Result<Vec<RawTransaction>> {
        let mut txs = vec![];

        let mut buffer = Vec::new();
        OpTxEnvelope::Deposit(TxDeposit {
            source_hash: DEPLOY_FJORD_GAS_PRICE_ORACLE_SOURCE.source_hash(),
            from: GAS_PRICE_ORACLE_FJORD_DEPLOYER_ADDRESS,
            to: TxKind::Create,
            mint: 0.into(),
            value: U256::ZERO,
            gas_limit: 1_450_000,
            is_system_transaction: false,
            input: FjordTransactionBuilder::gas_price_oracle_deployment_bytecode(),
        })
        .encode_2718(&mut buffer);
        txs.push(RawTransaction::from(buffer));

        // Update the gas price oracle proxy.
        buffer = Vec::new();
        OpTxEnvelope::Deposit(TxDeposit {
            source_hash: UPDATE_FJORD_GAS_PRICE_ORACLE_SOURCE.source_hash(),
            from: Address::ZERO,
            to: TxKind::Call(GAS_PRICE_ORACLE_ADDRESS),
            mint: 0.into(),
            value: U256::ZERO,
            gas_limit: 50_000,
            is_system_transaction: false,
            input: upgrade_to_calldata(FJORD_GAS_PRICE_ORACLE_ADDRESS),
        })
        .encode_2718(&mut buffer);
        txs.push(RawTransaction::from(buffer));

        // Enable Fjord
        buffer = Vec::new();
        OpTxEnvelope::Deposit(TxDeposit {
            source_hash: ENABLE_FJORD_SOURCE.source_hash(),
            from: L1_INFO_DEPOSITER_ADDRESS,
            to: TxKind::Call(GAS_PRICE_ORACLE_ADDRESS),
            mint: 0.into(),
            value: U256::ZERO,
            gas_limit: 90_000,
            is_system_transaction: false,
            input: SET_FJORD_METHOD_SIGNATURE.into(),
        })
        .encode_2718(&mut buffer);
        txs.push(RawTransaction::from(buffer));

        Ok(txs)
    }
}

impl FjordTransactionBuilder {
    /// Returns the fjord gas price oracle deployment bytecode.
    pub const fn gas_price_oracle_deployment_bytecode() -> Bytes {
        bytes!("608060405234801561001057600080fd5b506117f6806100206000396000f3fe608060405234801561001057600080fd5b50600436106101365760003560e01c80636ef25c3a116100b2578063de26c4a111610081578063f45e65d811610066578063f45e65d81461025b578063f820614014610263578063fe173b971461020d57600080fd5b8063de26c4a114610235578063f1c7a58b1461024857600080fd5b80636ef25c3a1461020d5780638e98b10614610213578063960e3a231461021b578063c59859181461022d57600080fd5b806349948e0e11610109578063519b4bd3116100ee578063519b4bd31461019f57806354fd4d50146101a757806368d5dca6146101f057600080fd5b806349948e0e1461016f5780634ef6e2241461018257600080fd5b80630c18c1621461013b57806322b90ab3146101565780632e0f262514610160578063313ce56714610168575b600080fd5b61014361026b565b6040519081526020015b60405180910390f35b61015e61038c565b005b610143600681565b6006610143565b61014361017d3660046112a1565b610515565b60005461018f9060ff1681565b604051901515815260200161014d565b610143610552565b6101e36040518060400160405280600581526020017f312e332e3000000000000000000000000000000000000000000000000000000081525081565b60405161014d9190611370565b6101f86105b3565b60405163ffffffff909116815260200161014d565b48610143565b61015e610638565b60005461018f90610100900460ff1681565b6101f8610832565b6101436102433660046112a1565b610893565b6101436102563660046113e3565b61098d565b610143610a69565b610143610b5c565b6000805460ff1615610304576040517f08c379a000000000000000000000000000000000000000000000000000000000815260206004820152602860248201527f47617350726963654f7261636c653a206f76657268656164282920697320646560448201527f707265636174656400000000000000000000000000000000000000000000000060648201526084015b60405180910390fd5b73420000000000000000000000000000000000001573ffffffffffffffffffffffffffffffffffffffff16638b239f736040518163ffffffff1660e01b8152600401602060405180830381865afa158015610363573d6000803e3d6000fd5b505050506040513d601f19601f8201168201806040525081019061038791906113fc565b905090565b3373deaddeaddeaddeaddeaddeaddeaddeaddead000114610455576040517f08c379a000000000000000000000000000000000000000000000000000000000815260206004820152604160248201527f47617350726963654f7261636c653a206f6e6c7920746865206465706f73697460448201527f6f72206163636f756e742063616e2073657420697345636f746f6e6520666c6160648201527f6700000000000000000000000000000000000000000000000000000000000000608482015260a4016102fb565b60005460ff16156104e8576040517f08c379a000000000000000000000000000000000000000000000000000000000815260206004820152602660248201527f47617350726963654f7261636c653a2045636f746f6e6520616c72656164792060448201527f616374697665000000000000000000000000000000000000000000000000000060648201526084016102fb565b600080547fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff00166001179055565b60008054610100900460ff16156105355761052f82610bbd565b92915050565b60005460ff16156105495761052f82610bdc565b61052f82610c80565b600073420000000000000000000000000000000000001573ffffffffffffffffffffffffffffffffffffffff16635cf249696040518163ffffffff1660e01b8152600401602060405180830381865afa158015610363573d6000803e3d6000fd5b600073420000000000000000000000000000000000001573ffffffffffffffffffffffffffffffffffffffff166368d5dca66040518163ffffffff1660e01b8152600401602060405180830381865afa158015610614573d6000803e3d6000fd5b505050506040513d601f19601f820116820180604052508101906103879190611415565b3373deaddeaddeaddeaddeaddeaddeaddeaddead0001146106db576040517f08c379a000000000000000000000000000000000000000000000000000000000815260206004820152603f60248201527f47617350726963654f7261636c653a206f6e6c7920746865206465706f73697460448201527f6f72206163636f756e742063616e20736574206973466a6f726420666c61670060648201526084016102fb565b60005460ff1661076d576040517f08c379a000000000000000000000000000000000000000000000000000000000815260206004820152603960248201527f47617350726963654f7261636c653a20466a6f72642063616e206f6e6c79206260448201527f65206163746976617465642061667465722045636f746f6e650000000000000060648201526084016102fb565b600054610100900460ff1615610804576040517f08c379a0000000000000000000000000000000000000000000000000000000008152602060048201526024808201527f47617350726963654f7261636c653a20466a6f726420616c726561647920616360448201527f746976650000000000000000000000000000000000000000000000000000000060648201526084016102fb565b600080547fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff00ff16610100179055565b600073420000000000000000000000000000000000001573ffffffffffffffffffffffffffffffffffffffff1663c59859186040518163ffffffff1660e01b8152600401602060405180830381865afa158015610614573d6000803e3d6000fd5b60008054610100900460ff16156108da57620f42406108c56108b484610dd4565b516108c090604461146a565b6110f1565b6108d0906010611482565b61052f91906114bf565b60006108e583611150565b60005490915060ff16156108f95792915050565b73420000000000000000000000000000000000001573ffffffffffffffffffffffffffffffffffffffff16638b239f736040518163ffffffff1660e01b8152600401602060405180830381865afa158015610958573d6000803e3d6000fd5b505050506040513d601f19601f8201168201806040525081019061097c91906113fc565b610986908261146a565b9392505050565b60008054610100900460ff16610a25576040517f08c379a000000000000000000000000000000000000000000000000000000000815260206004820152603660248201527f47617350726963654f7261636c653a206765744c314665655570706572426f7560448201527f6e64206f6e6c7920737570706f72747320466a6f72640000000000000000000060648201526084016102fb565b6000610a3283604461146a565b90506000610a4160ff836114bf565b610a4b908361146a565b610a5690601061146a565b9050610a61816111e0565b949350505050565b6000805460ff1615610afd576040517f08c379a000000000000000000000000000000000000000000000000000000000815260206004820152602660248201527f47617350726963654f7261636c653a207363616c61722829206973206465707260448201527f656361746564000000000000000000000000000000000000000000000000000060648201526084016102fb565b73420000000000000000000000000000000000001573ffffffffffffffffffffffffffffffffffffffff16639e8c49666040518163ffffffff1660e01b8152600401602060405180830381865afa158015610363573d6000803e3d6000fd5b600073420000000000000000000000000000000000001573ffffffffffffffffffffffffffffffffffffffff1663f82061406040518163ffffffff1660e01b8152600401602060405180830381865afa158015610363573d6000803e3d6000fd5b600061052f610bcb83610dd4565b51610bd790604461146a565b6111e0565b600080610be883611150565b90506000610bf4610552565b610bfc610832565b610c079060106114fa565b63ffffffff16610c179190611482565b90506000610c23610b5c565b610c2b6105b3565b63ffffffff16610c3b9190611482565b90506000610c49828461146a565b610c539085611482565b9050610c616006600a611646565b610c6c906010611482565b610c7690826114bf565b9695505050505050565b600080610c8c83611150565b9050600073420000000000000000000000000000000000001573ffffffffffffffffffffffffffffffffffffffff16639e8c49666040518163ffffffff1660e01b8152600401602060405180830381865afa158015610cef573d6000803e3d6000fd5b505050506040513d601f19601f82011682018060405250810190610d1391906113fc565b610d1b610552565b73420000000000000000000000000000000000001573ffffffffffffffffffffffffffffffffffffffff16638b239f736040518163ffffffff1660e01b8152600401602060405180830381865afa158015610d7a573d6000803e3d6000fd5b505050506040513d601f19601f82011682018060405250810190610d9e91906113fc565b610da8908561146a565b610db29190611482565b610dbc9190611482565b9050610dca6006600a611646565b610a6190826114bf565b6060610f63565b818153600101919050565b600082840393505b838110156109865782810151828201511860001a1590930292600101610dee565b825b60208210610e5b578251610e26601f83610ddb565b52602092909201917fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe090910190602101610e11565b8115610986578251610e706001840383610ddb565b520160010192915050565b60006001830392505b6101078210610ebc57610eae8360ff16610ea960fd610ea98760081c60e00189610ddb565b610ddb565b935061010682039150610e84565b60078210610ee957610ee28360ff16610ea960078503610ea98760081c60e00189610ddb565b9050610986565b610a618360ff16610ea98560081c8560051b0187610ddb565b610f5b828203610f3f610f2f84600081518060001a8160011a60081b178160021a60101b17915050919050565b639e3779b90260131c611fff1690565b8060021b6040510182815160e01c1860e01b8151188152505050565b600101919050565b6180003860405139618000604051016020830180600d8551820103826002015b81811015611096576000805b50508051604051600082901a600183901a60081b1760029290921a60101b91909117639e3779b9810260111c617ffc16909101805160e081811c878603811890911b90911890915284019081830390848410610feb5750611026565b600184019350611fff8211611020578251600081901a600182901a60081b1760029190911a60101b1781036110205750611026565b50610f8f565b838310611034575050611096565b600183039250858311156110525761104f8787888603610e0f565b96505b611066600985016003850160038501610de6565b9150611073878284610e7b565b96505061108b8461108686848601610f02565b610f02565b915050809350610f83565b50506110a88383848851850103610e0f565b925050506040519150618000820180820391508183526020830160005b838110156110dd5782810151828201526020016110c5565b506000920191825250602001604052919050565b60008061110183620cc394611482565b61112b907ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffd763200611652565b905061113b6064620f42406116c6565b81121561052f576109866064620f42406116c6565b80516000908190815b818110156111d35784818151811061117357611173611782565b01602001517fff00000000000000000000000000000000000000000000000000000000000000166000036111b3576111ac60048461146a565b92506111c1565b6111be60108461146a565b92505b806111cb816117b1565b915050611159565b50610a618261044061146a565b6000806111ec836110f1565b905060006111f8610b5c565b6112006105b3565b63ffffffff166112109190611482565b611218610552565b611220610832565b61122b9060106114fa565b63ffffffff1661123b9190611482565b611245919061146a565b905061125360066002611482565b61125e90600a611646565b6112688284611482565b610a6191906114bf565b7f4e487b7100000000000000000000000000000000000000000000000000000000600052604160045260246000fd5b6000602082840312156112b357600080fd5b813567ffffffffffffffff808211156112cb57600080fd5b818401915084601f8301126112df57600080fd5b8135818111156112f1576112f1611272565b604051601f82017fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe0908116603f0116810190838211818310171561133757611337611272565b8160405282815287602084870101111561135057600080fd5b826020860160208301376000928101602001929092525095945050505050565b600060208083528351808285015260005b8181101561139d57858101830151858201604001528201611381565b818111156113af576000604083870101525b50601f017fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe016929092016040019392505050565b6000602082840312156113f557600080fd5b5035919050565b60006020828403121561140e57600080fd5b5051919050565b60006020828403121561142757600080fd5b815163ffffffff8116811461098657600080fd5b7f4e487b7100000000000000000000000000000000000000000000000000000000600052601160045260246000fd5b6000821982111561147d5761147d61143b565b500190565b6000817fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff04831182151516156114ba576114ba61143b565b500290565b6000826114f5577f4e487b7100000000000000000000000000000000000000000000000000000000600052601260045260246000fd5b500490565b600063ffffffff8083168185168183048111821515161561151d5761151d61143b565b02949350505050565b600181815b8085111561157f57817fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff048211156115655761156561143b565b8085161561157257918102915b93841c939080029061152b565b509250929050565b6000826115965750600161052f565b816115a35750600061052f565b81600181146115b957600281146115c3576115df565b600191505061052f565b60ff8411156115d4576115d461143b565b50506001821b61052f565b5060208310610133831016604e8410600b8410161715611602575081810a61052f565b61160c8383611526565b807fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff0482111561163e5761163e61143b565b029392505050565b60006109868383611587565b6000808212827f7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff0384138115161561168c5761168c61143b565b827f80000000000000000000000000000000000000000000000000000000000000000384128116156116c0576116c061143b565b50500190565b60007f7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff6000841360008413858304851182821616156117075761170761143b565b7f800000000000000000000000000000000000000000000000000000000000000060008712868205881281841616156117425761174261143b565b6000871292508782058712848416161561175e5761175e61143b565b878505871281841616156117745761177461143b565b505050929093029392505050565b7f4e487b7100000000000000000000000000000000000000000000000000000000600052603260045260246000fd5b60007fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff82036117e2576117e261143b565b506001019056fea164736f6c634300080f000a")
    }
}
