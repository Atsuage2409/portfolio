import tkinter as tk
from tkinter import ttk
import csv
import datetime
import os

class ScoreManager:
    def __init__(self, filename="kyudo_result.csv"):
        self.filename = filename

        # 列定義：左から 射手1 → 射手5
        self.headers = ["回数", "射手1", "射手2", "射手3", "射手4", "射手5"]

        # 行定義：上から 4射目 → 1射目
        self.row_labels = ["4射目", "3射目", "2射目", "1射目"]

        # GUI初期化
        self.root = tk.Tk()
        self.root.title("的中状況モニタ")
        self.root.geometry("500x200")

        self.tree = ttk.Treeview(
            self.root,
            columns=self.headers,
            show="headings",
            height=4
        )

        for col in self.headers:
            self.tree.heading(col, text=col)
            self.tree.column(col, width=80, anchor="center")

        self.tree.pack(fill="both", expand=True)

        # 初期行挿入
        for i, label in enumerate(self.row_labels):
            self.tree.insert(
                "",
                "end",
                iid=i,
                values=(label, "-", "-", "-", "-", "-")
            )

        # CSVヘッダー作成
        if not os.path.exists(self.filename):
            with open(self.filename, mode="w", newline="", encoding="utf-8-sig") as f:
                writer = csv.writer(f)
                writer.writerow(
                    ["Timestamp", "射手", "1射目", "2射目", "3射目", "4射目"]
                )

    def update_gui(self, scoreboard):
        """
        scoreboard[row][col]
        row: 0=4射目 → 3=1射目
        col: 0=射手1 → 4=射手5
        """

        for row in range(4):
            label = self.row_labels[row]
            row_data = [label]

            for col in range(5):
                try:
                    val = scoreboard[row][col]
                except IndexError:
                    val = "-"
                row_data.append(val)

            self.tree.item(row, values=row_data)

        self.root.update_idletasks()
        self.root.update()

    def save_csv(self, scoreboard):
        """
        scoreboard[row][col] を
        CSV: 射手ごと・射順1→4で保存
        """
        now_str = datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")

        try:
            with open(self.filename, mode="a", newline="", encoding="utf-8-sig") as f:
                writer = csv.writer(f)

                for col in range(5):
                    shooter = f"射手{col+1}"
                    shots = ["-"] * 4

                    for row in range(4):
                        # row 3 が 1射目
                        shots[3 - row] = scoreboard[row][col]

                    writer.writerow([now_str, shooter] + shots)

            print(f"Saved to {self.filename}")

        except Exception as e:
            print(f"Save Error: {e}")

    def close(self):
        self.root.destroy()
