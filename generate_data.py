import csv
import random

headers = ["Verbrauch", "Menge", "Einheit", "cost (EUR)", "Lieferant", "Datum", "Kommentar", "Scope", "Emissionsfaktor", "N/A", "", "Áramfogyasztás", " ", "Company ID"]

activities = [
    ("Erdgas", "m3", "Scope 1"), ("Diesel", "L", "Scope 1"), ("Heizöl", "L", "Scope 1"), ("Benzin", "L", "Scope 1"),
    ("Strom", "kWh", "Scope 2"), ("Fernwärme", "MWh", "Scope 2"),
    ("Beschaffung", "EUR", "Scope 3"), ("Eingangsfracht", "tkm", "Scope 3"), ("Dienstreise", "km", "Scope 3"), 
    ("Pendeln", "km", "Scope 3"), ("Abfall", "kg", "Scope 3"), ("Investitionen", "EUR", "Scope 3"), ("Fuhrpark", "km", "Scope 3"),
    ("Beteiligungen", "EUR", "Finance"), ("Unternehmenskredite", "USD", "Finance"), ("Gewerbeimmobilien", "CHF", "Finance")
]

with open("targoo_stress_test.csv", "w", newline="", encoding="utf-8") as f:
    writer = csv.writer(f)
    writer.writerow(headers)
    for i in range(5000):
        act, unit, scope = random.choice(activities)
        
        r = random.random()
        if r < 0.05:
            val = "geschätzt"
        elif r < 0.10:
            val = f"{random.uniform(1, 1000):.2f}".replace(".", ",")
        else:
            val = str(round(random.uniform(1, 10000), 2))
            
        row = [
            val, # Verbrauch (Col 0 -> will be picked as value_col_idx fallback)
            str(round(random.uniform(1, 100), 2)), # Menge
            unit, # Einheit
            str(round(random.uniform(10, 500), 2)), # cost
            "Supplier X", # Lieferant
            "2024-01-01", # Datum
            act, # Kommentar (fallback will find activity here)
            scope, # Scope
            "0.5", # EF
            "na", # N/A
            "", # empty
            act if random.random() > 0.5 else "Other", # Áramfogyasztás
            " ", # space
            f"CID-{i}" # Company ID
        ]
        writer.writerow(row)

print("Generated targoo_stress_test.csv with 5000 rows.")
