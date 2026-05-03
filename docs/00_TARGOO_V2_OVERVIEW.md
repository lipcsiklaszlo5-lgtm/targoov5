# Targoo V2 – ESG Data Refinery Motor

## 1. A Projekt Célja (The "Why")
A Targoo V2 egy "headless" ESG adatnormalizáló és feldolgozó motor. A célja, hogy a freelancer tulajdonosa (Lipcsik László, ESG Data Process Architect) számára egy olyan "black box" szolgáltatást nyújtson, amivel a DACH-régió butik tanácsadóinak és KKV-inak a nyers, "koszos" vállalati adatait (Excel, CSV, ERP dump) egyetlen gombnyomásra, percek alatt, Big4-kompatibilis, auditbiztos jelentéscsomaggá (Fritz Package) tudja alakítani. A projekt NEM egy eladható SaaS szoftver, hanem egy szolgáltatás alapját képező, determinisztikus és auditálható adatfeldolgozó motor.

## 2. Hogyan Működik? (The "How")
A motor a következő lépéseken megy keresztül:
1. Ingestion: A nyers fájlok beolvasása.
2. Triage: Többnyelvű fejlécek felismerése.
3. Calculation: Scope 1, 2, 3 számítások.
4. Output: Fritz Package generálása.

## 3. Technológiai Stack
- Nyelv: Rust
- Web Framework: Axum
- Adatbázis: SQLite (WORM)

## 4. Fejlesztési Alapelvek
- Determinizmus
- Auditálhatóság
- Robusztusság (soha nem panic-el)
- "Never unwrap()"
