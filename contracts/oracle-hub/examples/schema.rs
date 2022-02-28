use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, export_schema_with_title, remove_schemas, schema_for};

use tefi_oracle::hub::{
    ConfigResponse, HubExecuteMsg, HubQueryMsg, InstantiateMsg, PriceListResponse, PriceResponse,
    ProxyWhitelistResponse, SourcesResponse, AllSourcesResponse, AssetSymbolMapResponse
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema_with_title(&schema_for!(HubExecuteMsg), &out_dir, "ExecuteMsg");
    export_schema_with_title(&schema_for!(HubQueryMsg), &out_dir, "QueryMsg");
    export_schema(&schema_for!(ConfigResponse), &out_dir);
    export_schema(&schema_for!(ProxyWhitelistResponse), &out_dir);
    export_schema(&schema_for!(PriceResponse), &out_dir);
    export_schema(&schema_for!(PriceListResponse), &out_dir);
    export_schema(&schema_for!(SourcesResponse), &out_dir);
    export_schema(&schema_for!(AllSourcesResponse), &out_dir);
    export_schema(&schema_for!(AssetSymbolMapResponse), &out_dir);
}
