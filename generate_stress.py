import csv
import random

headers = ["Verbrauch", "Menge", "Einheit", "cost (EUR)", "Lieferant", "Datum", "Kommentar", "Scope", "Emissionsfaktor", "N/A", "", "Áramfogyasztás", " ", "Company ID"]

activities = [
    ("Erdgas", "m3", "Scope 1"), ("Diesel", "L", "Scope 1"), ("Heizöl", "L", "Scope 1"),
    ("Strom", "kWh", "Scope 2"), ("Fernwärme", "MWh", "Scope 2"),
    ("Beschaffung", "EUR", "Scope 3"), ("Eingangsfracht", "tkm", "Scope 3"), ("Dienstreise", "km", "Scope 3"), 
    ("Pendeln", "km", "Scope 3"), ("Abfall", "kg", "Scope 3"), ("Investitionen", "EUR", "Scope 3"),
    ("Beteiligungen", "EUR", "Finance"), ("Unternehmenskredite", "USD", "Finance")
]

with open("targoo_stress_test.csv", "w", newline="", encoding="utf-8") as f:
    writer = csv.writer(f)
    writer.writerow(headers)
    for i in range(5000):
        act, unit, scope = random.choice(activities)
        val = str(round(random.uniform(1, 5000), 2))
        if random.random() < 0.05: val = "geschätzt" # "Dirty" data
        
        row = [
            val, # Verbrauch
            str(round(random.uniform(1, 100), 2)), # Menge
            unit, # Einheit
            str(round(random.uniform(10, 500), 2)), # cost
            "Supplier X", "2024-01-01", 
            act, # Kommentar (Context for triage)
            scope, "0.5", "na", "", 
            act if random.random() > 0.5 else "Other", # Áramfogyasztás (Context)
            " ", f"CID-{i}"
        ]
        writer.writerow(row)
