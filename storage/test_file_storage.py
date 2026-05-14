import pytest
import httpx
import uuid
from pathlib import Path
from io import BytesIO
from PIL import Image


BASE_URL = "http://localhost:3003"
TEST_BUCKET = "freelance-files"


@pytest.fixture(scope="session")
def client():
    """Синхронный HTTP-клиент для тестов"""
    with httpx.Client(base_url=BASE_URL, timeout=60.0) as c:
        yield c


@pytest.fixture
def test_image():
    """Генерирует тестовое изображение 1 МБ"""
    img = Image.new('RGB', (800, 600), color='blue')
    buf = BytesIO()
    img.save(buf, format='JPEG', quality=85)
    buf.seek(0)
    return ("test_image.jpg", buf.read(), "image/jpeg")


@pytest.fixture
def test_video():
    """Создаёт небольшой тестовый видео-файл (имитация)"""
    data = b"fake video content " * 1_000_000  # ~15 МБ
    return ("test_video.mp4", data, "video/mp4")


@pytest.fixture
def test_large_file():
    """Файл ~500 МБ для проверки лимита"""
    data = b"0" * (500 * 1024 * 1024)
    return ("large_file.zip", data, "application/zip")


# ===================== ТЕСТЫ =====================

def test_upload_image_permanent(client, test_image):
    """Загрузка изображения с permanent=True"""
    filename, data, content_type = test_image

    files = {"file": (filename, data, content_type)}
    response = client.post("/upload/image?permanent=true", files=files)

    assert response.status_code == 200
    json = response.json()
    assert json["success"] is True
    assert "link" in json
    assert "permanent" in json["key"]
    assert json["link"].startswith("http")


def test_upload_image_temporary(client, test_image):
    """Загрузка изображения без permanent (temporary)"""
    filename, data, content_type = test_image
    files = {"file": (filename, data, content_type)}

    response = client.post("/upload/image", files=files)
    assert response.status_code == 200
    assert "temporary" in response.json()["key"]


def test_upload_video_success(client, test_video):
    """Видео до 30 секунд — должно пройти"""
    filename, data, content_type = test_video
    files = {"file": (filename, data, content_type)}

    response = client.post("/upload/video", files=files)
    assert response.status_code == 200
    json = response.json()
    assert "videos/temporary" in json["key"]

def test_upload_general_file_compressed(client):
    """Общий файл с сжатием"""
    files = {"file": ("document.pdf", b"%PDF-1.4 fake pdf content" * 1000, "application/pdf")}
    response = client.post("/upload/file?permanent=true", files=files)  # permanent=true = compress
    assert response.status_code == 200
    assert response.json()["success"] is True


def test_upload_general_file_no_compression(client):
    """Общий файл без сжатия"""
    files = {"file": ("archive.zip", b"fake zip" * 5000, "application/zip")}
    response = client.post("/upload/file?permanent=false", files=files)  # false = no compression
    assert response.status_code == 200


#def test_file_too_large(client):
#    """Превышение лимита 8 ГБ"""
#    huge_data = b"0" * (9 * 1024 * 1024 * 1024)  # 9 ГБ
#    files = {"file": ("huge.bin", huge_data, "application/octet-stream")}
#    response = client.post("/upload/file", files=files)
#    assert response.status_code in (413, 400)  # Payload Too Large или Bad Request


def test_delete_file(client, test_image):
    """Полный цикл: загрузка -> удаление"""
    filename, data, content_type = test_image
    files = {"file": (filename, data, content_type)}

    # Загружаем
    upload_resp = client.post("/upload/image?permanent=true", files=files)
    assert upload_resp.status_code == 200
    key = upload_resp.json()["key"]
    print(key)

    # Удаляем
    delete_resp = client.delete(f"/files/{key}")
    assert delete_resp.status_code == 200


def test_unsupported_mime_image(client):
    """Неподдерживаемый MIME для изображения"""
    files = {"file": ("text.txt", b"not an image", "text/plain")}
    response = client.post("/upload/image", files=files)
    assert response.status_code == 400


if __name__ == "__main__":
    pytest.main(["-v", "--tb=short"])

