# Changelog

## [Unreleased]
### Added
- 2026-05-02: A Gemini CLI sikeresen beolvasta és értelmezte a Targoo V2 projekt átfogó dokumentációját.
- 2026-05-03: Teljes kód audit végrehajtva. 25 figyelmeztetést, 19 unwrap() hívást, hiányzó CLI kapcsolókat és kritikus versenyhelyzetet találtunk a szótárkezelésnél.
- 2026-05-03: Javítva a kritikus versenyhelyzet a szótárkezelésnél (fájl zárolás hozzáadva) és egy unwrap() hívás a triage_context.rs-ben.
- 2026-05-03: Javítva az összes unwrap()/expect() hívás a main.rs-ben (3 db) és az api.rs-ben (2 db). Minden teszt sikeresen lefutott.
- 2026-05-03: Javítva az összes unwrap()/expect() hívás a gemini_client.rs-ben (3 db) és az output_factory.rs-ben (2 db).
- 2026-05-03: Implementálva a headless mód (--input, --scope3-only, --dictionary-only, --output kapcsolók). A motor most már API nélkül, parancssorból futtatható.
- 2026-05-03: Streaming ingest implementálva az ingest.rs-ben. A motor most már soronként dolgozza fel a fájlokat, jelentősen csökkentve a memóriahasználatot. A RawRow Arc<str>-t használ a redundáns string allokációk elkerülésére. A régi batch függvény deprecated lett.
- 2026-05-03: Teljes körű funkcionális tesztelés végrehajtva 5 tesztfájlon. 5/5 sikeres futtatás, a Fritz Package ZIP-ek minden tesztesetben létrejöttek. A motor stabil, de a szótár bővítése szükséges a felismerési arány javításához.
- 2026-05-03: Szótár lecserélve az új, 10.500 bejegyzéses többnyelvű változatra. A régi szótár biztonsági mentése: data/dictionary_backup_20260503.json
- 2026-05-03: Nem szabványos elnevezések javítva (SCOPE2_LB→Scope2Lb, SCOPE2_MB→Scope2Mb). Használatlan importok és halott kód eltávolítva. A figyelmeztetések száma 11-re csökkent.
- 2026-05-03: Új emissziós faktor adatbázis létrehozva (data/efactors/database.json). DEFRA 2024, EPA 2024, eGRID 2023, USEEIO v2.1.
- 2026-05-03: Új emissziós faktor adatbázis integrálva a motorba (src/ef_database.rs). A motor mostantól a DEFRA 2024, EPA 2024, eGRID 2023 és USEEIO v2.1 hivatalos faktorait használja a hardkódolt értékek helyett.
- 2026-05-03: Implementálva a Schema Registry (src/schema_registry.rs) és az elsődleges SAP export séma (schemas/registry/sample_sap_export.json) a rugalmas és verziózott adatfeldolgozás támogatására.
