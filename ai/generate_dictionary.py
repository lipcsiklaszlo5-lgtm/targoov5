import pandas as pd
import json
import os
import re

def clean_keyword(text):
    if not isinstance(text, str): return ""
    # Remove any sequence of 5 or more digits anywhere in the text (DEFRA IDs)
    text = re.sub(r'\d{5,}', '', text)
    text = re.sub(r'\(.*?\)', '', text)
    text = re.sub(r'[^a-zA-Z0-9\s\-\/]', '', text)
    # Clean up multiple spaces
    text = re.sub(r'\s+', ' ', text)
    return text.strip()

def process_epa():
    print("Processing EPA...")
    path = "ai/sources/epa_2024.xlsx"
    xl = pd.ExcelFile(path)
    entries = []
    full_df = xl.parse("Emission Factors Hub")
    
    # Table 1: Fuel Combustion
    for i, row in full_df.iterrows():
        if "Table 1" in str(row.iloc[1]):
            fuel_df = full_df.iloc[i+4:i+50]
            for _, f_row in fuel_df.iterrows():
                fuel = str(f_row.iloc[1])
                if fuel == "nan" or "kg CO2" not in str(f_row.iloc[2]): break
                try:
                    entries.append({
                        "keyword": clean_keyword(fuel),
                        "ghg_category": "Scope1",
                        "canonical_unit": "mmbtu",
                        "ef_value": float(f_row.iloc[2]),
                        "ef_unit": "kg CO2/mmBtu",
                        "industry": "General",
                        "languages": ["en"],
                        "confidence_default": 0.95
                    })
                except: continue
    return entries

def process_defra():
    print("Processing DEFRA...")
    entries = []
    target_sheets = [
        'Fuels', 'Passenger vehicles', 'Delivery vehicles', 'UK electricity', 
        'Material use', 'Waste disposal', 'Business travel- air', 
        'Business travel- land', 'Freighting goods', 'Hotel stay',
        'UK electricity T&D for EVs', 'Water supply', 'Water treatment'
    ]
    
    cat_map = {
        'Fuels': 3, 'Passenger vehicles': 6, 'Delivery vehicles': 4,
        'UK electricity': 3, 'Material use': 1, 'Waste disposal': 5,
        'Business travel- air': 6, 'Business travel- land': 6,
        'Freighting goods': 4, 'Hotel stay': 6, 'Water supply': 1,
        'Water treatment': 1
    }

    for year in ["2024", "2025"]:
        path = f"ai/sources/defra_{year}.xlsx"
        if not os.path.exists(path): continue
        xl = pd.ExcelFile(path)
        for sheet in target_sheets:
            if sheet not in xl.sheet_names: continue
            print(f"  Parsing DEFRA {year} - {sheet}")
            
            temp_df = xl.parse(sheet, nrows=20)
            header_row = 0
            for i, row in temp_df.iterrows():
                row_str = " ".join(str(v).lower() for v in row.values)
                if "unit" in row_str and ("total" in row_str or "kg" in row_str):
                    header_row = i + 1
                    break
            
            df = xl.parse(sheet, skiprows=header_row)
            
            unit_col, factor_col, desc_cols = None, None, []
            for col in df.columns:
                c_idx = df.columns.get_loc(col)
                col_str = str(col).lower()
                if "unit" in col_str: unit_col = col
                if "total" in col_str and "ghg" in col_str: factor_col = col
                if c_idx < 4 and "unit" not in col_str and "ghg" not in col_str:
                    desc_cols.append(col)
            
            if not factor_col:
                for col in df.columns:
                    if "kg" in str(col).lower() and "co2" in str(col).lower():
                        factor_col = col; break

            if factor_col:
                for _, row in df.iterrows():
                    try:
                        desc_parts = [str(row[c]) for c in desc_cols if pd.notna(row[c]) and "Unnamed" not in str(c)]
                        desc = " ".join(desc_parts)
                        if not desc.strip() or desc.lower() == "nan" or "total" in desc.lower(): continue
                        
                        kw = clean_keyword(desc)
                        if not kw: continue

                        val = float(row[factor_col])
                        unit = str(row[unit_col]) if unit_col else "unit"
                        
                        entries.append({
                            "keyword": kw,
                            "ghg_category": "Scope3",
                            "scope3_id": cat_map.get(sheet, 1), 
                            "canonical_unit": unit.lower(),
                            "ef_value": val,
                            "ef_unit": f"kg CO2e / {unit}",
                            "industry": "General",
                            "languages": ["en"],
                            "confidence_default": 0.9
                        })
                    except: continue
    return entries

def process_exiobase():
    print("Processing EXIOBASE...")
    entries = []
    try:
        with open("ai/sources/exiobase_minimal/products.txt", "r") as f:
            products = [line.strip().split("\t")[-1] for line in f.readlines()]
        
        with open("ai/sources/exiobase_minimal/F.txt", "r") as f:
            f.readline()
            co2_line = ""
            for line in f:
                if "CO2" in line and "combustion" in line:
                    co2_line = line
                    break
            
            if co2_line:
                values = co2_line.strip().split("\t")[1:]
                for i, val in enumerate(values):
                    if i >= len(products): break
                    try:
                        entries.append({
                            "keyword": clean_keyword(products[i]),
                            "ghg_category": "Scope3",
                            "scope3_id": 1,
                            "calc_path": "SpendBased",
                            "canonical_unit": "eur", 
                            "ef_value": float(val) / 1000000.0,
                            "ef_unit": "kg CO2 / EUR",
                            "industry": "General",
                            "languages": ["en"],
                            "confidence_default": 0.7
                        })
                    except: continue
    except Exception as e:
        print(f"Error EXIOBASE: {e}")
    return entries

def main():
    all_entries = []
    all_entries.extend(process_epa())
    all_entries.extend(process_defra())
    all_entries.extend(process_exiobase())
    
    seen = set()
    unique_entries = []
    for e in all_entries:
        if not e['keyword'] or len(e['keyword']) < 3: continue
        if e['keyword'] not in seen:
            unique_entries.append(e)
            seen.add(e['keyword'])
            
    print(f"Total unique entries generated: {len(unique_entries)}")
    
    with open("ai/dictionary.json", "w") as f:
        json.dump(unique_entries, f, indent=2)
    with open("data/dictionary.json", "w") as f:
        json.dump(unique_entries, f, indent=2)

if __name__ == "__main__":
    main()
