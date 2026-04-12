import csv
import random

headers = ["Asset Class", "Financed Amount", "Currency"]
activities = ["investment portfolio", "business loan", "commercial real estate"]

with open("test_100.csv", "w", newline="", encoding="utf-8") as f:
    writer = csv.writer(f)
    writer.writerow(headers)
    for i in range(100):
        writer.writerow([random.choice(activities), random.uniform(1000, 1000000), "USD"])
