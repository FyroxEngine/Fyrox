pub const CRATE_NAME: &str = "myth-axiom";
pub const CREST: &str = "Axiom";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Comparator { Eq, NotEq, Lt, Lte, Gt, Gte, Contains, StartsWith, EndsWith }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum LogicOp { And, Or, Not, Xor, Nand }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RuleTarget { Actor, Faction, World, Item, Location, Any }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Condition {
    pub condition_id: String,
    pub label: String,
    pub field_path: String,         // dot-separated path into the data: "actor.health"
    pub comparator: Comparator,
    pub value: serde_json::Value,
    pub target: RuleTarget,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RuleExpression {
    pub op: LogicOp,
    pub condition_ids: Vec<String>, // leaf conditions
    pub sub_expressions: Vec<RuleExpression>, // nested
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Rule {
    pub rule_id: String,
    pub label: String,
    pub expression: RuleExpression,
    pub on_true_event: Option<String>,
    pub on_false_event: Option<String>,
    pub priority: u8,
    pub enabled: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AxiomConfig {
    pub rules: Vec<Rule>,
    pub eval_order: Vec<String>,    // rule_ids in evaluation order
    pub short_circuit: bool,        // stop evaluating on first match
    pub max_rules: u16,             // cap to prevent runaway complexity
    pub cache_results: bool,        // cache unchanged evaluations per tick
    pub strict_mode: bool,          // error on missing field_path vs. silently false
}

impl Default for AxiomConfig {
    fn default() -> Self {
        Self {
            rules: vec![],
            eval_order: vec![],
            short_circuit: true,
            max_rules: 256,
            cache_results: true,
            strict_mode: false,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RuleResult {
    pub rule_id: String,
    pub result: bool,
    pub evaluated_at: f64,
}
