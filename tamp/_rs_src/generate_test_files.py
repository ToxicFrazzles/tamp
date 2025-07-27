import tamp
from pathlib import Path

HERE = Path(__file__).parent
TEST_FILE_DIR = HERE / "test_files"


def generate_test_files(data: str, filename: str, window: int, literal: int) -> None:
    """Generates a test file with the given data."""
    uncompressed_file_path = TEST_FILE_DIR / f"{filename}_{window}_{literal}.txt"
    compressed_file_path = TEST_FILE_DIR / f"{filename}_{window}_{literal}.tamp"
    with uncompressed_file_path.open("w") as uf:
        uf.write(data)
    with tamp.open(compressed_file_path, "w", window=window, literal=literal) as cf:
        cf.write(data)

if __name__ == "__main__":
    # Clear the test files directory
    if TEST_FILE_DIR.exists():
        for file in TEST_FILE_DIR.iterdir():
            file.unlink()
    else:
        TEST_FILE_DIR.mkdir(parents=True)

    # Each of these test cases will generate a file with the specified data.
    test_cases = [
        ("Sample test data", "small"),
        ("A" * 1000, "large"),
        ("This is a test with special characters: !@#$%^&*()", "special_chars"),
    ]
    # Every pair of (window, literal) settings will generate a file for each test case.
    test_settings = [
        (10, 8),
        (15, 7),
        (11, 8),
        (8, 7),
    ]
    for data, filename in test_cases:
        for window, literal in test_settings:
            generate_test_files(data, filename, window, literal)
