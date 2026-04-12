use crate::models::{CalcPath, MatchMethod, Scope3Category};
use strsim::normalized_levenshtein;

pub struct Scope3Classifier {
    category_keywords: Vec<CategoryKeywordSet>,
}

#[derive(Clone)]
struct CategoryKeywordSet {
    cat_id: u8,
    cat_name: &'static str,
    default_calc_path: CalcPath,
    keywords_en: Vec<&'static str>,
    keywords_de: Vec<&'static str>,
    keywords_hu: Vec<&'static str>,
}

impl Scope3Classifier {
    pub fn new() -> Self {
        let categories = vec![
            CategoryKeywordSet {
                cat_id: 1,
                cat_name: "Purchased Goods & Services",
                default_calc_path: CalcPath::SpendBased,
                keywords_en: vec![
                    "purchase", "procurement", "supplier", "material", "vendor", "goods",
                    "services", "raw material", "component", "buy", "sub-contractor", "outsource",
                    "third party", "supply chain spend", "indirect spend", "direct material",
                ],
                keywords_de: vec![
                    "beschaffung", "einkauf", "lieferant", "rohmaterial", "vorleistung",
                    "zulieferer", "zukaufteile", "fremdleistung", "indirekter einkauf",
                    "direkte materialkosten",
                ],
                keywords_hu: vec![
                    "beszerzés", "anyag", "szállító", "nyersanyag", "alvállalkozó", "alkatrész",
                    "vásárlás", "közvetlen anyag",
                ],
            },
            CategoryKeywordSet {
                cat_id: 2,
                cat_name: "Capital Goods",
                default_calc_path: CalcPath::SpendBased,
                keywords_en: vec![
                    "capex", "capital expenditure", "capital goods", "equipment", "machinery",
                    "vehicle fleet", "building", "infrastructure", "asset purchase",
                    "investment goods", "plant", "hardware", "it hardware", "server",
                    "manufacturing equipment",
                ],
                keywords_de: vec![
                    "investitionsgüter", "kapitalanlage", "sachanlagen", "maschinen", "fuhrpark",
                    "fahrzeugkauf", "it-infrastruktur", "gebäude", "anlagegüter", "capex",
                ],
                keywords_hu: vec![
                    "tárgyi eszköz", "beruházás", "gépbeszerzés", "jármű vásárlás", "it eszköz",
                    "ingatlan vétel", "capex",
                ],
            },
            CategoryKeywordSet {
                cat_id: 3,
                cat_name: "Fuel & Energy Related Activities",
                default_calc_path: CalcPath::ActivityBased,
                keywords_en: vec![
                    "upstream energy", "fuel extraction", "transmission loss", "t&d loss",
                    "well-to-tank", "wtt", "fuel supply chain", "energy upstream", "grid loss",
                    "extraction emission",
                ],
                keywords_de: vec![
                    "vorgelagerter energiebedarf", "übertragungsverlust", "brennstoffversorgung",
                    "well-to-tank", "netzverlustte", "upstream-energie",
                ],
                keywords_hu: vec![
                    "upstream energia", "hálózati veszteség", "üzemanyag kitermelés",
                    "energiahordozó upstream", "átviteli veszteség",
                ],
            },
            CategoryKeywordSet {
                cat_id: 4,
                cat_name: "Upstream Transportation & Distribution",
                default_calc_path: CalcPath::ActivityBased,
                keywords_en: vec![
                    "freight", "trucking", "shipping", "logistics", "transport cost", "haulage",
                    "carrier", "road transport", "rail freight", "sea freight", "air cargo",
                    "courier", "delivery", "distribution", "inbound logistics", "upstream transport",
                    "supply chain transport", "tonne-km", "tkm", "vehicle-km",
                ],
                keywords_de: vec![
                    "spedition", "fracht", "logistikkosten", "lkw-transport", "schienentransport",
                    "seefracht", "luftfracht", "kurier", "anlieferung", "eingangstransport",
                    "lieferverkehr", "fuhrkosten",
                ],
                keywords_hu: vec![
                    "fuvarozás", "szállítás", "logisztika", "teherszállítás", "fuvarköltség",
                    "bejövő szállítás", "vasúti szállítás", "tengeri fuvár", "légi teherszállítás",
                    "futár", "szállítási költség", "tonna-km",
                ],
            },
            CategoryKeywordSet {
                cat_id: 5,
                cat_name: "Waste Generated in Operations",
                default_calc_path: CalcPath::ActivityBased,
                keywords_en: vec![
                    "waste", "landfill", "recycling", "incineration", "composting",
                    "hazardous waste", "non-hazardous", "mixed waste", "organic waste",
                    "industrial waste", "waste water", "sewage", "waste disposal", "solid waste",
                    "waste treatment", "skip hire",
                ],
                keywords_de: vec![
                    "abfall", "deponierung", "recycling", "verbrennung", "kompostierung",
                    "sondermüll", "gewerbeabfall", "restmüll", "abwasser", "klärschlamm",
                    "entsorgungskosten",
                ],
                keywords_hu: vec![
                    "hulladék", "lerakó", "újrahasznosítás", "égetés", "komposztálás",
                    "veszélyes hulladék", "ipari hulladék", "szennyvíz", "hulladékkezelés",
                    "kommunális hulladék",
                ],
            },
            CategoryKeywordSet {
                cat_id: 6,
                cat_name: "Business Travel",
                default_calc_path: CalcPath::ActivityBased,
                keywords_en: vec![
                    "flight", "flights", "air travel", "business travel", "hotel", "accommodation",
                    "train travel", "taxi", "rental car", "hire car", "travel expense", "mileage",
                    "passenger km", "conference travel", "intercontinental", "domestic flight",
                    "international flight", "economy class", "business class", "first class",
                ],
                keywords_de: vec![
                    "dienstreise", "flug", "businesstravel", "hotel", "übernachtung", "bahn",
                    "mietwagen", "reisekosten", "kilometer", "inlandsflug", "auslandsflug",
                    "economy", "business class",
                ],
                keywords_hu: vec![
                    "üzleti utazás", "repülőjegy", "szálloda", "szállás", "vonat", "bérlő gép",
                    "utazási költség", "kilométer", "belföldi repülő", "külföldi repülő",
                    "economy", "business class", "éjszaka",
                ],
            },
            CategoryKeywordSet {
                cat_id: 7,
                cat_name: "Employee Commuting",
                default_calc_path: CalcPath::ActivityBased,
                keywords_en: vec![
                    "commuting", "commute", "employee travel", "work from home", "wfh",
                    "remote work", "staff travel", "mileage allowance", "travel allowance",
                    "public transport subsidy", "season ticket", "shuttle bus", "headcount",
                    "fte", "employees",
                ],
                keywords_de: vec![
                    "pendeln", "mitarbeiterfahrten", "homeoffice", "fahrtkostenzuschuss",
                    "pendlerpauschale", "firmenshuttle", "mitarbeiterzahl", "vollzeitäquivalent",
                ],
                keywords_hu: vec![
                    "ingázás", "munkavállalói utazás", "home office", "pendlerpauschale",
                    "létszám", "fte", "céges buszjárat", "utazási támogatás",
                ],
            },
            CategoryKeywordSet {
                cat_id: 8,
                cat_name: "Upstream Leased Assets",
                default_calc_path: CalcPath::SpendBased,
                keywords_en: vec![
                    "leased asset", "operating lease", "leased property", "rented office",
                    "rented warehouse", "leased vehicle", "leased equipment", "lease cost",
                    "leasing", "lessor", "rental", "leasehold",
                ],
                keywords_de: vec![
                    "leasingobjekt", "gemietete immobilie", "betriebsleasing", "leasingkosten",
                    "mietobjekt", "fahrzeugleasing", "betriebsliegenschaft",
                ],
                keywords_hu: vec![
                    "bérelt eszköz", "operatív lízing", "bérelt ingatlan", "bérelt jármű",
                    "lízingköltség", "lízing", "bérlet",
                ],
            },
            CategoryKeywordSet {
                cat_id: 9,
                cat_name: "Downstream Transportation & Distribution",
                default_calc_path: CalcPath::ActivityBased,
                keywords_en: vec![
                    "outbound logistics", "downstream transport", "customer delivery",
                    "distribution", "product distribution", "delivery cost", "outbound freight",
                    "last mile", "warehousing", "distribution centre", "fulfillment",
                    "order fulfillment",
                ],
                keywords_de: vec![
                    "auslieferung", "ausgangstransport", "kundendistribution", "lagerkosten",
                    "distributionszentrum", "sendungskosten", "lieferung",
                ],
                keywords_hu: vec![
                    "kiszállítás", "kimenő logisztika", "vevői szállítás", "disztribúció",
                    "raktározás", "elosztó", "szállítási díj", "kézbesítés",
                ],
            },
            CategoryKeywordSet {
                cat_id: 10,
                cat_name: "Processing of Sold Products",
                default_calc_path: CalcPath::SpendBased,
                keywords_en: vec![
                    "processing", "downstream processing", "sold intermediate",
                    "value chain processing", "customer manufacturing", "processing cost",
                    "product transformation", "semi-finished", "intermediate product",
                ],
                keywords_de: vec![
                    "weiterverarbeitung", "zwischenprodukt", "nachgelagerte verarbeitung",
                    "halbfabrikat", "verarbeitungskosten",
                ],
                keywords_hu: vec![
                    "feldolgozás", "féltermék", "downstream feldolgozás", "vevői feldolgozás",
                    "intermedier termék", "félkész termék",
                ],
            },
            CategoryKeywordSet {
                cat_id: 11,
                cat_name: "Use of Sold Products",
                default_calc_path: CalcPath::ActivityBased,
                keywords_en: vec![
                    "product use", "use phase", "use of sold products", "product lifetime",
                    "end-use consumption", "sold appliances", "product energy use", "unit sold",
                    "consumer use", "in-use emission", "product lifecycle",
                    "operational emission",
                ],
                keywords_de: vec![
                    "produktnutzung", "nutzungsphase", "verkaufte produkte verwendung",
                    "nutzungsemissionen", "produktlebensdauer", "verbrauch beim kunden",
                    "geräteenergie",
                ],
                keywords_hu: vec![
                    "termék használat", "értékesített termék", "életciklus", "használati fázis",
                    "fogyasztói használat", "energiafelhasználás termékhasználat során",
                    "üzemi kibocsátás",
                ],
            },
            CategoryKeywordSet {
                cat_id: 12,
                cat_name: "End-of-Life Treatment of Sold Products",
                default_calc_path: CalcPath::ActivityBased,
                keywords_en: vec![
                    "end of life", "eol", "product disposal", "product recycling",
                    "product landfill", "waste treatment", "take-back scheme", "product waste",
                    "weee", "packaging waste", "post-consumer waste",
                ],
                keywords_de: vec![
                    "produktentsorgung", "lebensende", "eol-behandlung", "produktabfall",
                    "rücknahme", "verpackungsentsorgung", "weee", "nachnutzung", "entsorgung",
                ],
                keywords_hu: vec![
                    "termék hulladék", "életciklus vége", "eol", "termék visszavétel",
                    "csomagolási hulladék", "elektromos hulladék", "hulladékkezelés",
                ],
            },
            CategoryKeywordSet {
                cat_id: 13,
                cat_name: "Downstream Leased Assets",
                default_calc_path: CalcPath::ActivityBased,
                keywords_en: vec![
                    "downstream leased", "leased to customers", "assets leased out",
                    "property leased", "fleet leasing", "equipment leasing", "rental income asset",
                    "landlord emissions", "tenant occupied",
                ],
                keywords_de: vec![
                    "verleaste anlagen", "vermietete immobilien", "flottenleasing",
                    "mietergebäude", "vermietung", "leasingnehmer-emissionen",
                ],
                keywords_hu: vec![
                    "bérbe adott eszköz", "bérbe adott ingatlan", "flotta lízing",
                    "bérbevevő kibocsátása", "bérbeadói emisszió",
                ],
            },
            CategoryKeywordSet {
                cat_id: 14,
                cat_name: "Franchises",
                default_calc_path: CalcPath::SpendBased,
                keywords_en: vec![
                    "franchise", "franchisee", "franchise network", "franchise operations",
                    "franchise fee", "franchise partner", "franchise outlet", "sub-franchise",
                ],
                keywords_de: vec![
                    "franchise", "franchisenehmer", "franchisesystem", "franchisepartner",
                    "franchisenetz", "lizenzgeber",
                ],
                keywords_hu: vec![
                    "franchise", "franchisee", "franchise hálózat", "franchise partner",
                    "franchise díj", "licenszpartner",
                ],
            },
            CategoryKeywordSet {
                cat_id: 15,
                cat_name: "Investments",
                default_calc_path: CalcPath::Pcaf,
                keywords_en: vec![
                    "investment", "portfolio", "financial asset", "loan", "bond", "equity",
                    "fund", "aum", "financed emissions", "pcaf", "attribution", "carbon intensity",
                    "waci", "project finance", "real estate fund", "infrastructure investment",
                    "private equity", "venture capital", "asset under management",
                ],
                keywords_de: vec![
                    "investitionen", "portfolio", "finanzvermögen", "kredit", "anleihe",
                    "beteiligung", "fonds", "finanzierte emissionen", "pcaf", "kapitalintensität",
                    "infrastrukturinvestition",
                ],
                keywords_hu: vec![
                    "befektetés", "portfólió", "pénzügyi eszköz", "hitel", "kötvény",
                    "részesedés", "alap", "finanszírozott emisszió", "pcaf", "tőkearányos intenzitás",
                    "projektfinanszírozás",
                ],
            },
        ];

        Self {
            category_keywords: categories,
        }
    }

    /// Classifies a header string to a specific Scope 3 category
    pub fn classify(&self, header: &str) -> Option<ClassificationResult> {
        let normalized = header.to_lowercase();
        
        // 1. Exact keyword match
        for cat_set in &self.category_keywords {
            let all_keywords: Vec<&str> = cat_set
                .keywords_en
                .iter()
                .chain(cat_set.keywords_de.iter())
                .chain(cat_set.keywords_hu.iter())
                .cloned()
                .collect();

            for keyword in all_keywords {
                if normalized.contains(keyword) {
                    return Some(ClassificationResult {
                        cat_id: cat_set.cat_id,
                        cat_name: cat_set.cat_name.to_string(),
                        calc_path: cat_set.default_calc_path,
                        match_method: MatchMethod::Exact,
                        confidence: 1.0,
                        matched_keyword: keyword.to_string(),
                    });
                }
            }
        }

        // 2. Fuzzy match (Levenshtein)
        let mut best_match: Option<(u8, &'static str, CalcPath, f64, String)> = None;
        
        for cat_set in &self.category_keywords {
            let all_keywords: Vec<&str> = cat_set
                .keywords_en
                .iter()
                .chain(cat_set.keywords_de.iter())
                .chain(cat_set.keywords_hu.iter())
                .cloned()
                .collect();

            for keyword in all_keywords {
                // Check similarity for each word in the header
                for header_word in normalized.split_whitespace() {
                    let score = normalized_levenshtein(header_word, keyword);
                    if score > best_match.as_ref().map(|(_, _, _, s, _)| *s).unwrap_or(0.0) && score >= 0.85 {
                        best_match = Some((
                            cat_set.cat_id,
                            cat_set.cat_name,
                            cat_set.default_calc_path,
                            score,
                            keyword.to_string(),
                        ));
                    }
                }
            }
        }

        if let Some((cat_id, cat_name, calc_path, score, keyword)) = best_match {
            return Some(ClassificationResult {
                cat_id,
                cat_name: cat_name.to_string(),
                calc_path,
                match_method: MatchMethod::Fuzzy,
                confidence: score as f32,
                matched_keyword: keyword,
            });
        }

        // 3. Currency fallback -> Cat 1 SpendBased
        if self.is_currency_header(&normalized) {
            return Some(ClassificationResult {
                cat_id: 1,
                cat_name: "Purchased Goods & Services".to_string(),
                calc_path: CalcPath::SpendBased,
                match_method: MatchMethod::Inferred,
                confidence: 0.5,
                matched_keyword: "currency_heuristic".to_string(),
            });
        }

        None
    }

    fn is_currency_header(&self, normalized: &str) -> bool {
        let currency_keywords = [
            "usd", "eur", "gbp", "huf", "ft", "$", "€", "£", "cost", "spend", "price", "amount", "betrag",
            "kosten", "preis", "summe", "összeg", "ár", "költség",
        ];
        currency_keywords.iter().any(|kw| normalized.contains(kw))
    }
}

impl Default for Scope3Classifier {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ClassificationResult {
    pub cat_id: u8,
    pub cat_name: String,
    pub calc_path: CalcPath,
    pub match_method: MatchMethod,
    pub confidence: f32,
    pub matched_keyword: String,
}
