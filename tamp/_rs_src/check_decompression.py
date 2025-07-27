import tamp
from pathlib import Path

HERE = Path(__file__).parent



if __name__ == "__main__":
    with tamp.open(HERE / "debug_compressed_output.tamp", "r") as f:
        data = f.read()
    print(data)
