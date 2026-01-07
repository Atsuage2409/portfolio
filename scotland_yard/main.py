import random
import json

with open("map.json", "r", encoding="utf-8") as f:
    MapData = json.load(f)

def get_num(min_value, prompt_message):
    while True:
        user_input = input(prompt_message)
        try:
            number = int(user_input)
            if number >= min_value:
                return number
            else:
                print(f"エラー: {min_value}以上の値を入力してください。")
        except ValueError:
            print("エラー: 整数値を入力してください。")

def can_move(positions, position, card):
    Candidate_taxi = MapData[str(position)]["taxi"]
    Candidate_bus = MapData[str(position)]["bus"]
    Candidate_ug = MapData[str(position)]["underground"]
    result_taxi = [item for item in Candidate_taxi if item not in positions]

class Player:
    def __init__(self, name, position):
        self.name = name
        self.position = position
        pass

class Game:
    def __init__(self):
        self.num_players = 0
        self.players = []
        self.positions = []
        pass
    def start(self):
        num_players = get_num(2,"参加人数を入力してください。")
        self.players = [Player(f"プレイヤー{i + 1}") for i in range(num_players)]
        for player in self.players:
            player.name = input()
            player.position = random.randint(1,199)
    def get_position(self):
        self.positions = []
        for player in self.players:
            self.positions.append(player.position)