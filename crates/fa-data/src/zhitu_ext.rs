//! ZhituAPI extended feature modules.
//!
//! This module provides [`ZhituClient`], a standalone HTTP client for the ZhituAPI
//! Chinese stock data service. It covers endpoints beyond the core `DataProvider`
//! trait (which only exposes `fetch_quote` and `fetch_ohlcv`), including stock
//! lists, trading pools, company details, real-time trading data, and technical
//! indicators.

use std::collections::HashMap;

use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

const DEFAULT_BASE_URL: &str = "https://api.zhituapi.com";

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// A standalone HTTP client for the ZhituAPI service.
///
/// This is intentionally separate from the core `ZhituProvider` (which
/// implements the `DataProvider` trait) so that consumers can access the full
/// breadth of ZhituAPI endpoints without going through the trait abstraction.
pub struct ZhituClient {
    client: Client,
    base_url: String,
    token: String,
}

impl ZhituClient {
    /// Create a new client with the given API token.
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: DEFAULT_BASE_URL.to_string(),
            token: token.into(),
        }
    }

    /// Override the base URL (useful for testing with a mock server).
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    // -- URL helpers --------------------------------------------------------

    fn url(&self, path: &str) -> String {
        format!("{}{}?token={}", self.base_url, path, self.token)
    }

    fn url_with_params(&self, path: &str, params: &[(&str, &str)]) -> String {
        let mut url = self.url(path);
        for (k, v) in params {
            url.push_str(&format!("&{}={}", k, v));
        }
        url
    }

    // -- Internal request helper --------------------------------------------

    async fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T, String> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }
        resp.json::<T>().await.map_err(|e| e.to_string())
    }

    // =======================================================================
    // Category 1: Stock Lists (股票列表)
    // =======================================================================

    /// 所有股票列表 — `GET /hs/list/all`
    pub async fn list_all_stocks(&self) -> Result<Vec<StockInfo>, String> {
        self.get_json(&self.url("/hs/list/all")).await
    }

    /// 新股列表 — `GET /hs/list/new`
    pub async fn list_new_stocks(&self) -> Result<Vec<NewStockInfo>, String> {
        self.get_json(&self.url("/hs/list/new")).await
    }

    /// 风险警示股 — `GET /hs/list/fx`
    pub async fn list_risk_stocks(&self) -> Result<Vec<StockInfo>, String> {
        self.get_json(&self.url("/hs/list/fx")).await
    }

    /// 板块列表 — `GET /hs/list/sectors`
    pub async fn list_sectors(&self) -> Result<Vec<SectorInfo>, String> {
        self.get_json(&self.url("/hs/list/sectors")).await
    }

    /// 一级板块 — `GET /hs/list/primary`
    pub async fn list_primary_sectors(&self) -> Result<Vec<PrimarySector>, String> {
        self.get_json(&self.url("/hs/list/primary")).await
    }

    /// 板块成分股 — `GET /hs/sectors/{name}`
    pub async fn list_sector_stocks(&self, sector_name: &str) -> Result<Vec<StockInfo>, String> {
        let path = format!("/hs/sectors/{}", sector_name);
        self.get_json(&self.url(&path)).await
    }

    // =======================================================================
    // Category 2: Indices & Sectors (指数、行业、概念)
    // =======================================================================

    /// 指数树 — `GET /hs/index/tree`
    pub async fn index_tree(&self) -> Result<Vec<IndexNode>, String> {
        self.get_json(&self.url("/hs/index/tree")).await
    }

    /// 指数/板块成分股 — `GET /hs/index/stock/{code}`
    pub async fn index_stocks(&self, code: &str) -> Result<Vec<StockInfo>, String> {
        let path = format!("/hs/index/stock/{}", code);
        self.get_json(&self.url(&path)).await
    }

    /// 股票所属指数/板块 — `GET /hs/index/index/{code}`
    pub async fn stock_indices(&self, stock_code: &str) -> Result<Vec<IndexMembership>, String> {
        let path = format!("/hs/index/index/{}", stock_code);
        self.get_json(&self.url(&path)).await
    }

    // =======================================================================
    // Category 3: Trading Pools (涨跌股池)
    // =======================================================================

    /// 涨停股池 — `GET /hs/pool/ztgc/{date}`
    pub async fn pool_limit_up(&self, date: &str) -> Result<Vec<PoolStock>, String> {
        let path = format!("/hs/pool/ztgc/{}", date);
        self.get_json(&self.url(&path)).await
    }

    /// 跌停股池 — `GET /hs/pool/dtgc/{date}`
    pub async fn pool_limit_down(&self, date: &str) -> Result<Vec<PoolStock>, String> {
        let path = format!("/hs/pool/dtgc/{}", date);
        self.get_json(&self.url(&path)).await
    }

    /// 强势股池 — `GET /hs/pool/qsgc/{date}`
    pub async fn pool_strong(&self, date: &str) -> Result<Vec<PoolStock>, String> {
        let path = format!("/hs/pool/qsgc/{}", date);
        self.get_json(&self.url(&path)).await
    }

    /// 次新股池 — `GET /hs/pool/cxgc/{date}`
    pub async fn pool_sub_new(&self, date: &str) -> Result<Vec<PoolStock>, String> {
        let path = format!("/hs/pool/cxgc/{}", date);
        self.get_json(&self.url(&path)).await
    }

    /// 炸板股池 — `GET /hs/pool/zbgc/{date}`
    pub async fn pool_broken_board(&self, date: &str) -> Result<Vec<PoolStock>, String> {
        let path = format!("/hs/pool/zbgc/{}", date);
        self.get_json(&self.url(&path)).await
    }

    // =======================================================================
    // Category 4: Company Details (上市公司详情)
    // =======================================================================

    /// 公司简介 — `GET /gs/gsjj/{code}`
    pub async fn company_profile(&self, code: &str) -> Result<CompanyProfile, String> {
        let path = format!("/gs/gsjj/{}", code);
        self.get_json(&self.url(&path)).await
    }

    /// 所属指数 — `GET /gs/sszs/{code}`
    pub async fn company_indices(&self, code: &str) -> Result<Vec<Subsidiary>, String> {
        let path = format!("/gs/sszs/{}", code);
        self.get_json(&self.url(&path)).await
    }

    /// 高管履历 — `GET /gs/ljgg/{code}`
    pub async fn company_executives(&self, code: &str) -> Result<Vec<Executive>, String> {
        let path = format!("/gs/ljgg/{}", code);
        self.get_json(&self.url(&path)).await
    }

    /// 董事履历 — `GET /gs/ljds/{code}`
    pub async fn company_directors(&self, code: &str) -> Result<Vec<Executive>, String> {
        let path = format!("/gs/ljds/{}", code);
        self.get_json(&self.url(&path)).await
    }

    /// 监事履历 — `GET /gs/ljjs/{code}`
    pub async fn company_supervisors(&self, code: &str) -> Result<Vec<Executive>, String> {
        let path = format!("/gs/ljjs/{}", code);
        self.get_json(&self.url(&path)).await
    }

    /// 分红方案 — `GET /gs/jnff/{code}`
    pub async fn company_dividends(&self, code: &str) -> Result<Vec<DividendRecord>, String> {
        let path = format!("/gs/jnff/{}", code);
        self.get_json(&self.url(&path)).await
    }

    /// 增发/配股 — `GET /gs/jnzf/{code}`
    pub async fn company_additional_issues(
        &self,
        code: &str,
    ) -> Result<Vec<DividendDonation>, String> {
        let path = format!("/gs/jnzf/{}", code);
        self.get_json(&self.url(&path)).await
    }

    /// 基金募集 — `GET /gs/jjxs/{code}`
    pub async fn company_fund_raise(&self, code: &str) -> Result<Vec<FundRaise>, String> {
        let path = format!("/gs/jjxs/{}", code);
        self.get_json(&self.url(&path)).await
    }

    /// 业绩预告 — `GET /gs/yjyg/{code}`
    pub async fn company_profit_forecast(
        &self,
        code: &str,
    ) -> Result<Vec<ProfitForecast>, String> {
        let path = format!("/gs/yjyg/{}", code);
        self.get_json(&self.url(&path)).await
    }

    /// 经营范围 — `GET /gs/jyfw/{code}`
    pub async fn company_business_scope(
        &self,
        code: &str,
    ) -> Result<Vec<BusinessScope>, String> {
        let path = format!("/gs/jyfw/{}", code);
        self.get_json(&self.url(&path)).await
    }

    /// 季度利润 — `GET /gs/jdlr/{code}`
    pub async fn company_quarterly_profit(
        &self,
        code: &str,
    ) -> Result<Vec<QuarterlyProfit>, String> {
        let path = format!("/gs/jdlr/{}", code);
        self.get_json(&self.url(&path)).await
    }

    /// 季度现金流 — `GET /gs/jdxj/{code}`
    pub async fn company_quarterly_cashflow(
        &self,
        code: &str,
    ) -> Result<Vec<QuarterlyCashflow>, String> {
        let path = format!("/gs/jdxj/{}", code);
        self.get_json(&self.url(&path)).await
    }

    /// 财务指标 — `GET /gs/cwzb/{code}`
    pub async fn company_financial_indicators(
        &self,
        code: &str,
    ) -> Result<Vec<FinancialIndicators>, String> {
        let path = format!("/gs/cwzb/{}", code);
        self.get_json(&self.url(&path)).await
    }

    /// 十大股东 — `GET /gs/sdgd/{code}`
    pub async fn company_top_shareholders(
        &self,
        code: &str,
    ) -> Result<Vec<ShareholderReport>, String> {
        let path = format!("/gs/sdgd/{}", code);
        self.get_json(&self.url(&path)).await
    }

    /// 流通股东 — `GET /gs/ltgd/{code}`
    pub async fn company_float_shareholders(
        &self,
        code: &str,
    ) -> Result<Vec<ShareholderReport>, String> {
        let path = format!("/gs/ltgd/{}", code);
        self.get_json(&self.url(&path)).await
    }

    /// 股东变化 — `GET /gs/gdbh/{code}`
    pub async fn company_shareholder_changes(
        &self,
        code: &str,
    ) -> Result<Vec<ShareholderChange>, String> {
        let path = format!("/gs/gdbh/{}", code);
        self.get_json(&self.url(&path)).await
    }

    /// 基金持股 — `GET /gs/jjcg/{code}`
    pub async fn company_fund_holdings(
        &self,
        code: &str,
    ) -> Result<Vec<FundHolding>, String> {
        let path = format!("/gs/jjcg/{}", code);
        self.get_json(&self.url(&path)).await
    }

    // =======================================================================
    // Category 5: Real-time Trading (实时交易)
    // =======================================================================

    /// 实时交易(公开源) — `GET /hs/real/ssjy/{code}`
    pub async fn realtime_quote(&self, code: &str) -> Result<RealtimeQuote, String> {
        let path = format!("/hs/real/ssjy/{}", code);
        self.get_json(&self.url(&path)).await
    }

    /// 逐笔交易(公开源) — `GET /hs/real/zbjy/{code}`
    pub async fn realtime_ticks(&self, code: &str) -> Result<Vec<TickData>, String> {
        let path = format!("/hs/real/zbjy/{}", code);
        self.get_json(&self.url(&path)).await
    }

    /// 批量实时行情(包年/至尊) — `GET /hs/public/ssjymore?stock_codes={codes}`
    pub async fn realtime_batch(&self, codes: &[&str]) -> Result<Vec<RealtimeQuote>, String> {
        let joined = codes.join(",");
        let url = self.url_with_params("/hs/public/ssjymore", &[("stock_codes", &joined)]);
        self.get_json(&url).await
    }

    /// 分时数据(券商源) — `GET /hs/real/time/{code}`
    pub async fn realtime_time_share(&self, code: &str) -> Result<TimeShareData, String> {
        let path = format!("/hs/real/time/{}", code);
        self.get_json(&self.url(&path)).await
    }

    /// 五档盘口(券商源) — `GET /hs/real/five/{code}`
    pub async fn realtime_five_level(&self, code: &str) -> Result<FiveLevelOrder, String> {
        let path = format!("/hs/real/five/{}", code);
        self.get_json(&self.url(&path)).await
    }

    // =======================================================================
    // Category 6: Historical Data & Technical Indicators (行情数据、技术指标)
    // =======================================================================

    /// 历史成交明细 — `GET /hs/history/transaction/{code}?st={YYYYMMDD}&et={YYYYMMDD}`
    pub async fn transaction_history(
        &self,
        code: &str,
        start: &str,
        end: &str,
    ) -> Result<Vec<TransactionDetail>, String> {
        let path = format!("/hs/history/transaction/{}", code);
        let url = self.url_with_params(&path, &[("st", start), ("et", end)]);
        self.get_json(&url).await
    }

    /// 涨停跌停价历史 — `GET /hs/stopprice/history/{code}?st={YYYYMMDD}&et={YYYYMMDD}`
    pub async fn stop_price_history(
        &self,
        code: &str,
        start: &str,
        end: &str,
    ) -> Result<Vec<StopPrice>, String> {
        let path = format!("/hs/stopprice/history/{}", code);
        let url = self.url_with_params(&path, &[("st", start), ("et", end)]);
        self.get_json(&url).await
    }

    /// 技术指标 — `GET /hs/indicators/{code}?st={YYYYMMDD}&et={YYYYMMDD}`
    pub async fn technical_indicators(
        &self,
        code: &str,
        start: &str,
        end: &str,
    ) -> Result<Vec<TechnicalIndicator>, String> {
        let path = format!("/hs/indicators/{}", code);
        let url = self.url_with_params(&path, &[("st", start), ("et", end)]);
        self.get_json(&url).await
    }
}

// ===========================================================================
// Category 1 data types: Stock Lists (股票列表)
// ===========================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct StockInfo {
    #[serde(default)]
    pub dm: String, // 代码 (stock code)
    #[serde(default)]
    pub mc: String, // 名称 (name)
    #[serde(default)]
    pub jys: String, // 交易所 (exchange)
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewStockInfo {
    #[serde(default)]
    pub zqdm: String, // 证券代码
    #[serde(default)]
    pub zqjc: String, // 证券简称
    #[serde(default)]
    pub sgdm: String, // 申购代码
    #[serde(default)]
    pub fxsl: String, // 发行数量
    #[serde(default)]
    pub sgrq: String, // 申购日期
    #[serde(default)]
    pub fxjg: String, // 发行价格
    #[serde(default)]
    pub ssrq: String, // 上市日期
    #[serde(default)]
    pub syl: String, // 市盈率
    #[serde(default)]
    pub wszql: String, // 网上申购量
}

#[derive(Debug, Clone, Deserialize)]
pub struct SectorInfo {
    #[serde(default)]
    pub dm: String, // 板块代码
    #[serde(default)]
    pub mc: String, // 板块名称
    #[serde(default)]
    pub jys: String, // 交易所
}

#[derive(Debug, Clone, Deserialize)]
pub struct PrimarySector {
    #[serde(default)]
    pub mc: String, // 一级板块名称
}

// ===========================================================================
// Category 2 data types: Indices & Sectors (指数、行业、概念)
// ===========================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct IndexNode {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub type1: String,
    #[serde(default)]
    pub type2: String,
    #[serde(default)]
    pub level: String,
    #[serde(default)]
    pub pcode: String,
    #[serde(default)]
    pub pname: String,
    #[serde(default)]
    pub isleaf: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IndexMembership {
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub name: String,
}

// ===========================================================================
// Category 3 data types: Trading Pools (涨跌股池)
// ===========================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct PoolStock {
    #[serde(default)]
    pub dm: String, // 代码
    #[serde(default)]
    pub mc: String, // 名称
    #[serde(default)]
    pub p: String, // 价格 (price)
    #[serde(default)]
    pub zf: String, // 涨跌幅 (change %)
    #[serde(default)]
    pub cje: String, // 成交额 (turnover)
    #[serde(default)]
    pub lt: String, // 流通市值 (float market cap)
    #[serde(default)]
    pub zsz: String, // 总市值 (total market cap)
    #[serde(default)]
    pub hs: String, // 换手率 (turnover rate)
    #[serde(default)]
    pub lbc: String, // 连板次数 (consecutive limit count)
    #[serde(default)]
    pub fbt: String, // 封板/跌停时间
    #[serde(default)]
    pub lbt: String, // 连板天数
    #[serde(default)]
    pub zj: String, // 封单金额 (seal amount)
    #[serde(default)]
    pub zbc: String, // 炸板次数 (broken count)
    #[serde(default)]
    pub tj: String, // 统计
    #[serde(default)]
    pub pe: String, // 市盈率
    #[serde(default)]
    pub ztp: String, // 涨停价
    #[serde(default)]
    pub zs: String, // 涨速
    #[serde(default)]
    pub nh: String, // 新高
    #[serde(default)]
    pub lb: String, // 量比
    #[serde(default)]
    pub kb: String, // 开板日数
    #[serde(default)]
    pub od: String,
    #[serde(default)]
    pub ipod: String,
    #[serde(default)]
    pub zdf: String, // 振幅
}

// ===========================================================================
// Category 4 data types: Company Details (上市公司详情)
// ===========================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct CompanyProfile {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub ename: String,
    #[serde(default)]
    pub market: String,
    #[serde(default)]
    pub idea: String,
    #[serde(default)]
    pub ldate: String,
    #[serde(default)]
    pub sprice: String,
    #[serde(default)]
    pub principal: String,
    #[serde(default)]
    pub instype: String,
    #[serde(default)]
    pub secre: String,
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub bscope: String,
    #[serde(default)]
    pub printype: String,
    #[serde(default)]
    pub rprice: String,
    #[serde(default)]
    pub rtype: String,
    #[serde(default)]
    pub instype2: String,
    #[serde(default)]
    pub organ: String,
    #[serde(default)]
    pub lname: String,
    #[serde(default)]
    pub oname: String,
    #[serde(default)]
    pub rdprice: String,
    #[serde(default)]
    pub rnum: String,
    #[serde(default)]
    pub rprice2: String,
    #[serde(default)]
    pub rdnum: String,
    #[serde(default)]
    pub prov: String,
    #[serde(default)]
    pub area: String,
    #[serde(default)]
    pub industry: String,
    #[serde(default)]
    pub desc2: String,
    #[serde(default)]
    pub btype: String,
    #[serde(default)]
    pub tprice: String,
    #[serde(default)]
    pub tamount: String,
    #[serde(default)]
    pub tdesc: String,
    #[serde(default)]
    pub rdate: String,
    #[serde(default)]
    pub website: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub tel: String,
    #[serde(default)]
    pub fax: String,
    #[serde(default)]
    pub addr: String,
    #[serde(default)]
    pub desc3: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Executive {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub sdate: String,
    #[serde(default)]
    pub edate: String,
    #[serde(default)]
    pub sex: String,
    #[serde(default)]
    pub age: String,
    #[serde(default)]
    pub pay: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DividendRecord {
    #[serde(default)]
    pub sdate: String,
    #[serde(default)]
    pub give: String,
    #[serde(default)]
    pub change: String,
    #[serde(default)]
    pub send: String,
    #[serde(default)]
    pub line: String,
    #[serde(default)]
    pub cdate: String,
    #[serde(default)]
    pub edate: String,
    #[serde(default)]
    pub hdate: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DividendDonation {
    #[serde(default)]
    pub sdate: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub price: String,
    #[serde(default)]
    pub tprice: String,
    #[serde(default)]
    pub fprice: String,
    #[serde(default)]
    pub amount: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FundRaise {
    #[serde(default)]
    pub rdate: String,
    #[serde(default)]
    pub ramount: String,
    #[serde(default)]
    pub rprice: String,
    #[serde(default)]
    pub batch: String,
    #[serde(default)]
    pub pdate: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProfitForecast {
    #[serde(default)]
    pub pdate: String,
    #[serde(default)]
    pub rdate: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub abs: String,
    #[serde(default)]
    pub old: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BusinessScope {
    #[serde(default)]
    pub keyword: String,
    #[serde(default)]
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuarterlyProfit {
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub income: String,
    #[serde(default)]
    pub expend: String,
    #[serde(default)]
    pub profit: String,
    #[serde(default)]
    pub totalp: String,
    #[serde(default)]
    pub reprofit: String,
    #[serde(default)]
    pub basege: String,
    #[serde(default)]
    pub ettege: String,
    #[serde(default)]
    pub totalcp: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuarterlyCashflow {
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub jyin: String,
    #[serde(default)]
    pub jyout: String,
    #[serde(default)]
    pub jyfinal: String,
    #[serde(default)]
    pub tzin: String,
    #[serde(default)]
    pub tzout: String,
    #[serde(default)]
    pub tzfinal: String,
    #[serde(default)]
    pub czin: String,
    #[serde(default)]
    pub czout: String,
    #[serde(default)]
    pub czfinal: String,
    #[serde(default)]
    pub cashinc: String,
    #[serde(default)]
    pub cashs: String,
    #[serde(default)]
    pub cashe: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Shareholder {
    #[serde(default)]
    pub pm: String, // 排名
    #[serde(default)]
    pub gdmc: String, // 股东名称
    #[serde(default)]
    pub cgsl: String, // 持股数量
    #[serde(default)]
    pub cgbl: String, // 持股比例
    #[serde(default)]
    pub gbxz: String, // 股本性质
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShareholderReport {
    #[serde(default)]
    pub jzrq: String,
    #[serde(default)]
    pub ggrq: String,
    #[serde(default)]
    pub sdgd: Vec<Shareholder>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShareholderChange {
    #[serde(default)]
    pub jzrq: String,
    #[serde(default)]
    pub gdhs: String,
    #[serde(default)]
    pub bh: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FundHolding {
    #[serde(default)]
    pub jzrq: String,
    #[serde(default)]
    pub jjmc: String,
    #[serde(default)]
    pub jjdm: String,
    #[serde(default)]
    pub ccsl: String,
    #[serde(default)]
    pub ltbl: String,
    #[serde(default)]
    pub cgsz: String,
    #[serde(default)]
    pub jzbl: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Subsidiary {
    #[serde(default)]
    pub mc: String,
    #[serde(default)]
    pub dm: String,
    #[serde(default)]
    pub ind: String,
    #[serde(default)]
    pub outd: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FinancialIndicators {
    #[serde(default)]
    pub date: String,
    // Common financial ratio fields
    #[serde(default)]
    pub tbmg: String,
    #[serde(default)]
    pub jqmg: String,
    #[serde(default)]
    pub zclr: String,
    #[serde(default)]
    pub jzsy: String,
    #[serde(default)]
    pub ldbl: String,
    #[serde(default)]
    pub zcfzl: String,
    #[serde(default)]
    pub zzc: String,
    /// Remaining fields captured dynamically (~80+ possible keys).
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

// ===========================================================================
// Category 5 data types: Real-time Trading (实时交易)
// ===========================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct RealtimeQuote {
    #[serde(default)]
    pub fm: String, // 代码
    #[serde(default)]
    pub p: String, // 现价
    #[serde(default)]
    pub pc: String, // 昨收
    #[serde(default)]
    pub o: String, // 开盘
    #[serde(default)]
    pub h: String, // 最高
    #[serde(default)]
    pub l: String, // 最低
    #[serde(default)]
    pub v: String, // 成交量
    #[serde(default)]
    pub cje: String, // 成交额
    #[serde(default)]
    pub ud: String, // 涨跌额
    #[serde(default)]
    pub pe: String, // 市盈率
    #[serde(default)]
    pub zf: String, // 振幅
    #[serde(default)]
    pub sz: String, // 总市值
    #[serde(default)]
    pub hs: String, // 换手率
    #[serde(default)]
    pub lb: String, // 量比
    #[serde(default)]
    pub lt: String, // 流通市值
    #[serde(default)]
    pub yc: String, // 昨收(同pc)
    #[serde(default)]
    pub zs: String, // 涨速
    #[serde(default)]
    pub sjl: String, // 市净率
    #[serde(default)]
    pub zdf60: String, // 60日涨跌幅
    #[serde(default)]
    pub zdfnc: String, // 年初至今涨跌幅
    #[serde(default)]
    pub t: String, // 时间
}

#[derive(Debug, Clone, Deserialize)]
pub struct TickData {
    #[serde(default)]
    pub d: String, // 日期
    #[serde(default)]
    pub t: String, // 时间
    #[serde(default)]
    pub v: String, // 成交量
    #[serde(default)]
    pub p: String, // 价格
    #[serde(default)]
    pub ts: String, // 方向: 0=中性, 1=买入, 2=卖出
}

#[derive(Debug, Clone, Deserialize)]
pub struct TimeShareData {
    #[serde(default)]
    pub p: String, // 现价
    #[serde(default)]
    pub o: String, // 开盘
    #[serde(default)]
    pub h: String, // 最高
    #[serde(default)]
    pub l: String, // 最低
    #[serde(default)]
    pub yc: String, // 昨收
    #[serde(default)]
    pub cje: String, // 成交额
    #[serde(default)]
    pub v: String, // 成交量
    #[serde(default)]
    pub pv: String, // 量价
    #[serde(default)]
    pub ud: String, // 涨跌额
    #[serde(default)]
    pub pc: String, // 涨跌幅
    #[serde(default)]
    pub zf: String, // 振幅
    #[serde(default)]
    pub pe: String, // 市盈率
    #[serde(default)]
    pub tr: String, // 换手率
    #[serde(default)]
    pub pb_ratio: String, // 市净率
    #[serde(default)]
    pub tv: String, // 时间
}

#[derive(Debug, Clone, Deserialize)]
pub struct FiveLevelOrder {
    #[serde(default)]
    pub ps: Vec<String>, // 委卖价 (5 prices)
    #[serde(default)]
    pub pb: Vec<String>, // 委买价 (5 prices)
    #[serde(default)]
    pub vs: Vec<String>, // 委卖量
    #[serde(default)]
    pub vb: Vec<String>, // 委买量
    #[serde(default)]
    pub t: String, // 时间
}

// ===========================================================================
// Category 6 data types: Historical Data & Technical Indicators
// ===========================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct TransactionDetail {
    #[serde(default)]
    pub t: String,
    #[serde(default)]
    pub zmbzds: String,
    #[serde(default)]
    pub zmszds: String,
    #[serde(default)]
    pub dddx: String,
    #[serde(default)]
    pub zddy: String,
    #[serde(default)]
    pub ddcf: String,
    /// Remaining fields captured dynamically.
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StopPrice {
    #[serde(default)]
    pub t: String, // 日期
    #[serde(default)]
    pub h: String, // 涨停价
    #[serde(default)]
    pub l: String, // 跌停价
}

#[derive(Debug, Clone, Deserialize)]
pub struct TechnicalIndicator {
    #[serde(default)]
    pub time: String,
    #[serde(default)]
    pub lb: String,
    #[serde(default)]
    pub om: String,
    #[serde(default)]
    pub fm: String,
    #[serde(rename = "3d", default)]
    pub ma3d: String,
    #[serde(rename = "5d", default)]
    pub ma5d: String,
    #[serde(rename = "10d", default)]
    pub ma10d: String,
    #[serde(rename = "3t", default)]
    pub ma3t: String,
    #[serde(rename = "5t", default)]
    pub ma5t: String,
    #[serde(rename = "10t", default)]
    pub ma10t: String,
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    // -- Category 1: Stock Lists --------------------------------------------

    #[tokio::test]
    async fn test_list_all_stocks() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/hs/list/all")
            .match_query(mockito::Matcher::UrlEncoded(
                "token".into(),
                "test-token".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"dm":"600519","mc":"贵州茅台","jys":"SH"}]"#)
            .create_async()
            .await;

        let client = ZhituClient::new("test-token").with_base_url(server.url());
        let result = client.list_all_stocks().await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].dm, "600519");
        assert_eq!(result[0].mc, "贵州茅台");
        assert_eq!(result[0].jys, "SH");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_list_new_stocks() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/hs/list/new")
            .match_query(mockito::Matcher::UrlEncoded(
                "token".into(),
                "test-token".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[{"zqdm":"787001","zqjc":"华兴化学","sgdm":"787001","sgrq":"2025-01-10"}]"#,
            )
            .create_async()
            .await;

        let client = ZhituClient::new("test-token").with_base_url(server.url());
        let result = client.list_new_stocks().await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].zqdm, "787001");
        assert_eq!(result[0].zqjc, "华兴化学");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_list_sectors() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/hs/list/sectors")
            .match_query(mockito::Matcher::UrlEncoded(
                "token".into(),
                "test-token".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"dm":"BK0001","mc":"白酒","jys":"SH"}]"#)
            .create_async()
            .await;

        let client = ZhituClient::new("test-token").with_base_url(server.url());
        let result = client.list_sectors().await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].dm, "BK0001");
        mock.assert_async().await;
    }

    // -- Category 2: Indices & Sectors --------------------------------------

    #[tokio::test]
    async fn test_index_tree() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/hs/index/tree")
            .match_query(mockito::Matcher::UrlEncoded(
                "token".into(),
                "test-token".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[{"name":"上证指数","code":"000001","type1":"指数","type2":"","level":"1","pcode":"","pname":"","isleaf":"0"}]"#,
            )
            .create_async()
            .await;

        let client = ZhituClient::new("test-token").with_base_url(server.url());
        let result = client.index_tree().await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "上证指数");
        assert_eq!(result[0].code, "000001");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_stock_indices() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/hs/index/index/600519.SH")
            .match_query(mockito::Matcher::UrlEncoded(
                "token".into(),
                "test-token".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"code":"000001","name":"上证指数"}]"#)
            .create_async()
            .await;

        let client = ZhituClient::new("test-token").with_base_url(server.url());
        let result = client.stock_indices("600519.SH").await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].code, "000001");
        assert_eq!(result[0].name, "上证指数");
        mock.assert_async().await;
    }

    // -- Category 3: Trading Pools ------------------------------------------

    #[tokio::test]
    async fn test_pool_limit_up() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/hs/pool/ztgc/2025-01-15")
            .match_query(mockito::Matcher::UrlEncoded(
                "token".into(),
                "test-token".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[{"dm":"600519","mc":"贵州茅台","p":"1845.00","zf":"2.19"}]"#,
            )
            .create_async()
            .await;

        let client = ZhituClient::new("test-token").with_base_url(server.url());
        let result = client.pool_limit_up("2025-01-15").await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].dm, "600519");
        assert_eq!(result[0].p, "1845.00");
        assert_eq!(result[0].zf, "2.19");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_pool_limit_down() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/hs/pool/dtgc/2025-01-15")
            .match_query(mockito::Matcher::UrlEncoded(
                "token".into(),
                "test-token".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"dm":"000001","mc":"平安银行","p":"10.50","zf":"-10.00"}]"#)
            .create_async()
            .await;

        let client = ZhituClient::new("test-token").with_base_url(server.url());
        let result = client.pool_limit_down("2025-01-15").await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].dm, "000001");
        mock.assert_async().await;
    }

    // -- Category 4: Company Details ----------------------------------------

    #[tokio::test]
    async fn test_company_profile() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/gs/gsjj/600519.SH")
            .match_query(mockito::Matcher::UrlEncoded(
                "token".into(),
                "test-token".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"name":"贵州茅台","ename":"Kweichow Moutai","market":"SH","industry":"白酒"}"#,
            )
            .create_async()
            .await;

        let client = ZhituClient::new("test-token").with_base_url(server.url());
        let result = client.company_profile("600519.SH").await.unwrap();
        assert_eq!(result.name, "贵州茅台");
        assert_eq!(result.ename, "Kweichow Moutai");
        assert_eq!(result.industry, "白酒");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_company_executives() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/gs/ljgg/600519.SH")
            .match_query(mockito::Matcher::UrlEncoded(
                "token".into(),
                "test-token".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[{"name":"丁雄军","title":"董事长","sex":"男","age":"55","pay":"120.00"}]"#,
            )
            .create_async()
            .await;

        let client = ZhituClient::new("test-token").with_base_url(server.url());
        let result = client.company_executives("600519.SH").await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "丁雄军");
        assert_eq!(result[0].title, "董事长");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_company_dividends() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/gs/jnff/600519.SH")
            .match_query(mockito::Matcher::UrlEncoded(
                "token".into(),
                "test-token".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[{"sdate":"2024-06-15","give":"25.00","change":"","send":"","line":""}]"#,
            )
            .create_async()
            .await;

        let client = ZhituClient::new("test-token").with_base_url(server.url());
        let result = client.company_dividends("600519.SH").await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].give, "25.00");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_company_top_shareholders() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/gs/sdgd/600519.SH")
            .match_query(mockito::Matcher::UrlEncoded(
                "token".into(),
                "test-token".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[{"jzrq":"2024-09-30","ggrq":"2024-10-28","sdgd":[{"pm":"1","gdmc":"中国贵州茅台酒厂集团","cgsl":"678270000","cgbl":"54.00","gbxz":"国有法人"}]}]"#,
            )
            .create_async()
            .await;

        let client = ZhituClient::new("test-token").with_base_url(server.url());
        let result = client.company_top_shareholders("600519.SH").await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].sdgd.len(), 1);
        assert_eq!(result[0].sdgd[0].gdmc, "中国贵州茅台酒厂集团");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_company_financial_indicators() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/gs/cwzb/600519.SH")
            .match_query(mockito::Matcher::UrlEncoded(
                "token".into(),
                "test-token".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[{"date":"2024-09-30","tbmg":"49.50","jqmg":"42.30","zclr":"35.20","customfield":"1.23"}]"#,
            )
            .create_async()
            .await;

        let client = ZhituClient::new("test-token").with_base_url(server.url());
        let result = client
            .company_financial_indicators("600519.SH")
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].date, "2024-09-30");
        assert_eq!(result[0].tbmg, "49.50");
        // The extra field should be captured in the flatten map.
        assert!(result[0].extra.contains_key("customfield"));
        mock.assert_async().await;
    }

    // -- Category 5: Real-time Trading --------------------------------------

    #[tokio::test]
    async fn test_realtime_quote() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/hs/real/ssjy/600519.SH")
            .match_query(mockito::Matcher::UrlEncoded(
                "token".into(),
                "test-token".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"fm":"600519","p":"1845.00","pc":"1830.00","o":"1832.00","h":"1860.00","l":"1820.00","v":"12345678"}"#,
            )
            .create_async()
            .await;

        let client = ZhituClient::new("test-token").with_base_url(server.url());
        let result = client.realtime_quote("600519.SH").await.unwrap();
        assert_eq!(result.fm, "600519");
        assert_eq!(result.p, "1845.00");
        assert_eq!(result.pc, "1830.00");
        assert_eq!(result.h, "1860.00");
        assert_eq!(result.v, "12345678");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_realtime_ticks() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/hs/real/zbjy/600519.SH")
            .match_query(mockito::Matcher::UrlEncoded(
                "token".into(),
                "test-token".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[{"d":"2025-01-15","t":"09:30:03","v":"100","p":"1832.50","ts":"1"}]"#,
            )
            .create_async()
            .await;

        let client = ZhituClient::new("test-token").with_base_url(server.url());
        let result = client.realtime_ticks("600519.SH").await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].p, "1832.50");
        assert_eq!(result[0].ts, "1");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_realtime_batch() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/hs/public/ssjymore")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("token".into(), "test-token".into()),
                mockito::Matcher::UrlEncoded("stock_codes".into(), "600519.SH,000001.SZ".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[{"fm":"600519","p":"1845.00"},{"fm":"000001","p":"12.50"}]"#,
            )
            .create_async()
            .await;

        let client = ZhituClient::new("test-token").with_base_url(server.url());
        let result = client
            .realtime_batch(&["600519.SH", "000001.SZ"])
            .await
            .unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].fm, "600519");
        assert_eq!(result[1].fm, "000001");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_realtime_five_level() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/hs/real/five/600519.SH")
            .match_query(mockito::Matcher::UrlEncoded(
                "token".into(),
                "test-token".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"ps":["1850.00","1851.00","1852.00","1853.00","1854.00"],"pb":["1849.00","1848.00","1847.00","1846.00","1845.00"],"vs":["100","200","300","400","500"],"vb":["150","250","350","450","550"],"t":"09:30:05"}"#,
            )
            .create_async()
            .await;

        let client = ZhituClient::new("test-token").with_base_url(server.url());
        let result = client.realtime_five_level("600519.SH").await.unwrap();
        assert_eq!(result.ps.len(), 5);
        assert_eq!(result.pb.len(), 5);
        assert_eq!(result.ps[0], "1850.00");
        mock.assert_async().await;
    }

    // -- Category 6: Historical Data & Technical Indicators -----------------

    #[tokio::test]
    async fn test_technical_indicators() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/hs/indicators/600519.SH")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("token".into(), "test-token".into()),
                mockito::Matcher::UrlEncoded("st".into(), "20250101".into()),
                mockito::Matcher::UrlEncoded("et".into(), "20250115".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[{"time":"2025-01-15","lb":"1.05","om":"32.5","fm":"28.3","3d":"1840.00","5d":"1835.00","10d":"1820.00"}]"#,
            )
            .create_async()
            .await;

        let client = ZhituClient::new("test-token").with_base_url(server.url());
        let result = client
            .technical_indicators("600519.SH", "20250101", "20250115")
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].time, "2025-01-15");
        assert_eq!(result[0].ma3d, "1840.00");
        assert_eq!(result[0].ma5d, "1835.00");
        assert_eq!(result[0].ma10d, "1820.00");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_transaction_history() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/hs/history/transaction/600519.SH")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("token".into(), "test-token".into()),
                mockito::Matcher::UrlEncoded("st".into(), "20250101".into()),
                mockito::Matcher::UrlEncoded("et".into(), "20250115".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[{"t":"2025-01-15","zmbzds":"1234","zmszds":"5678","dddx":"0.5","zddy":"10.0","ddcf":"100"}]"#,
            )
            .create_async()
            .await;

        let client = ZhituClient::new("test-token").with_base_url(server.url());
        let result = client
            .transaction_history("600519.SH", "20250101", "20250115")
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].t, "2025-01-15");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_stop_price_history() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/hs/stopprice/history/600519.SH")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("token".into(), "test-token".into()),
                mockito::Matcher::UrlEncoded("st".into(), "20250101".into()),
                mockito::Matcher::UrlEncoded("et".into(), "20250115".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"t":"2025-01-15","h":"2029.50","l":"1645.50"}]"#)
            .create_async()
            .await;

        let client = ZhituClient::new("test-token").with_base_url(server.url());
        let result = client
            .stop_price_history("600519.SH", "20250101", "20250115")
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].h, "2029.50");
        assert_eq!(result[0].l, "1645.50");
        mock.assert_async().await;
    }

    // -- Error handling -----------------------------------------------------

    #[tokio::test]
    async fn test_http_error_returns_err() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/hs/list/all")
            .match_query(mockito::Matcher::UrlEncoded(
                "token".into(),
                "test-token".into(),
            ))
            .with_status(500)
            .create_async()
            .await;

        let client = ZhituClient::new("test-token").with_base_url(server.url());
        let result = client.list_all_stocks().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("HTTP 500"));
        mock.assert_async().await;
    }
}
