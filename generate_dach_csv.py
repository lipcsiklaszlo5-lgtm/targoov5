import csv
import random
from datetime import datetime, timedelta

def generate_dach_csv(filename):
    headers = [
        "Verbrauch", "Menge", "Einheit", "Kosten (EUR)", "Lieferant", "Datum", "Kommentar", 
        "Scope", "Emissionsfaktor", "N/A", "", "Áramfogyasztás", " ", "Company ID"
    ]
    
    rows = []
    
    # Probabilities
    # Scope 1 (25%): 'Erdgas', 'Diesel', 'Heizöl', 'Benzin', 'Kohle', 'Biomasse'
    # Scope 2 (25%): 'Strom', 'Fernwärme', 'Ökostrom'
    # Scope 3 Cat 1 (20%): 'Beschaffung', 'Einkauf', 'Rohmaterial', 'Lieferant A', 'Lieferant B', 'Supplier C'
    # Scope 3 Cat 3 (5%): 'Well-to-Tank', 'Vorgelagerte Energie'
    # Scope 3 Cat 4 (5%): 'Eingangsfracht', 'Seefracht', 'LKW-Transport'
    # Scope 3 Cat 6 (10%): 'Dienstreise', 'Flug', 'Hotel', 'Bahn'
    # Scope 3 Cat 7 (5%): 'Pendeln', 'Homeoffice'
    # Scope 3 Cat 15 (5%): 'Beteiligungen', 'Unternehmenskredite', 'Gewerbeimmobilien'

    categories = [
        ('Scope 1', ['Erdgas', 'Diesel', 'Heizöl', 'Benzin', 'Kohle', 'Biomasse'], ['kWh', 'L', 'm3', 'kg'], 250),
        ('Scope 2', ['Strom', 'Fernwärme', 'Ökostrom'], ['kWh', 'MWh'], 250),
        ('Scope 3 Cat 1', ['Beschaffung', 'Einkauf', 'Rohmaterial', 'Lieferant A', 'Lieferant B', 'Supplier C'], ['EUR', 'USD'], 200),
        ('Scope 3 Cat 3', ['Well-to-Tank', 'Vorgelagerte Energie'], ['kWh', 'L'], 50),
        ('Scope 3 Cat 4', ['Eingangsfracht', 'Seefracht', 'LKW-Transport'], ['tkm', 'km'], 50),
        ('Scope 3 Cat 6', ['Dienstreise', 'Flug', 'Hotel', 'Bahn'], ['km', 'pkm', 'night'], 100),
        ('Scope 3 Cat 7', ['Pendeln', 'Homeoffice'], ['km', 'hour'], 50),
        ('Scope 3 Cat 15', ['Beteiligungen', 'Unternehmenskredite', 'Gewerbeimmobilien'], ['EUR', 'USD'], 50)
    ]
    
    for scope, items, units, count in categories:
        for _ in range(count):
            item = random.choice(items)
            unit = random.choice(units)
            val = random.uniform(10.0, 10000.0)
            
            # Format value string with noise
            if random.random() < 0.1:
                val_str = f"{val:,.2f}".replace(',', 'X').replace('.', ',').replace('X', '.') # German format
            elif random.random() < 0.05:
                val_str = f"ca. {val:.1f}"
            else:
                val_str = f"{val:.2f}"
            
            comment = random.choice(["geschätzt", "genau", "Rechnung", "", ""])
            date = (datetime(2024, 1, 1) + timedelta(days=random.randint(0, 364))).strftime("%Y-%m-%d")
            
            row = [
                item, # Verbrauch
                val_str, # Menge
                unit, # Einheit
                f"{val * 0.5:.2f}" if unit not in ['EUR', 'USD'] else "", # Kosten (EUR)
                f"Supplier {random.randint(1, 20)}", # Lieferant
                date, # Datum
                comment, # Kommentar
                scope, # Scope
                f"{random.uniform(0.1, 2.5):.3f}", # Emissionsfaktor
                "N/A", # N/A
                "", # ""
                "Ja" if 'Strom' in item else "Nein", # Áramfogyasztás
                " ", # " "
                f"COMP-{random.randint(1000, 9999)}" # Company ID
            ]
            rows.append(row)
            
    # Add some completely empty rows as noise
    for _ in range(20):
        rows.append([""] * len(headers))
        
    random.shuffle(rows)
    
    with open(filename, 'w', newline='', encoding='utf-8') as f:
        writer = csv.writer(f)
        writer.writerow(headers)
        # Write exactly 1000 rows (discarding extra empty ones if any, though the logic above creates 1020)
        # Wait, the requirement says "Generálj pontosan 1000 sort".
        # Let's adjust to exactly 1000 data rows + 1 header row = 1001 rows.
        # So we won't add extra empty rows, but instead replace some existing rows with empty/noisy rows, 
        # or just make sure the total is 1000. 
        # The prompt says "Generálj pontosan 1000 sort", which implies data rows.
        # Let's just output the 1000 generated data rows.
        # Actually, adding noise might reduce the 1000 count if we replace them. 
        # I'll just write 1000 rows.
        for row in rows[:1000]:
            writer.writerow(row)

if __name__ == "__main__":
    generate_dach_csv("targoo_dach_final_test.csv")
