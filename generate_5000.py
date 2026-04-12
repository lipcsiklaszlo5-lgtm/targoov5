import csv
import random

headers = ["Asset Class", "Financed Amount", "Currency", "Header", "Value", "Unit"]
finance_activities = ["investment portfolio", "business loan", "commercial real estate"]
other_activities = ["electricity", "natural gas", "diesel fuel", "business travel", "waste management"]

with open("test_5000.csv", "w", newline="", encoding="utf-8") as f:
    writer = csv.writer(f)
    writer.writerow(headers)
    for i in range(5000):
        if random.random() > 0.2: # 80% other activities
            act = random.choice(other_activities)
            writer.writerow(["", 0, "", act, random.uniform(10, 10000), "kWh" if "electricity" in act else "L"])
        else: # 20% finance
            act = random.choice(finance_activities)
            writer.writerow([act, random.uniform(1000, 1000000), "USD", "", 0, ""])
