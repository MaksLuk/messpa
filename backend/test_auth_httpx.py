import httpx
import json
import time
from urllib.parse import urljoin

BASE_URL =  "http://10.104.41.159:443/api" #"http://127.0.0.1:3001"
GREEN = "\033[92m"
RED = "\033[91m"
RESET = "\033[0m"

def print_error(message: str, response=None):
    print(f"{RED}Ошибка: {message}{RESET}")
    if response is not None:
        print(f"Статус: {response.status_code}")
        print(response.text)

def test_tg_auth():
    print("=== Тестирование Auth API (tg) ===\n")
    with httpx.Client(verify=False) as session:
        print("1. Отправка кода через Telegram...")
        send_resp = session.post(urljoin(BASE_URL, "/api/auth/telegram/send-code"))
        if send_resp.status_code != 200:
            print_error("Не удалось отправить код", send_resp)
            return
        print(send_resp.text)

        print("\n2. Проверка кода (verify)...")
        code = input("\nВведите 6-значный код из Telegram: ").strip()
        verify_resp = session.post(urljoin(BASE_URL, "/api/auth/telegram/verify"), json={"magic_token": send_resp.json()['data']['magic_token'], "code": code})
        if verify_resp.status_code != 200:
            print_error("Ошибка верификации", verify_resp)
            return
        data = verify_resp.json()
        access_token = data.get("data", {}).get("access_token")
        user = data.get("data", {}).get("user", {})
        cookies = verify_resp.cookies      
        if not access_token:
            print_error("Не удалось получить access_token из ответа")
            return
        time.sleep(1)

        print(f"{GREEN}Успешная авторизация!{RESET}")
        print(f"   Пользователь: {user.get('display_name') or user.get('email')}")
        print(f"   Access Token получен")

        print("\n3. Получение информации о пользователе. Проверка /me")
        me_resp = session.get(urljoin(BASE_URL, "/api/user/me"), headers={"Authorization": f"Bearer {access_token}"})
        if me_resp.status_code == 200:
            print(f"{GREEN}/me работает корректно{RESET}")
            print("   Ответ:", json.dumps(me_resp.json(), indent=2, ensure_ascii=False))
        else:
            print_error("/me вернул ошибку", me_resp)
            return
        time.sleep(1)

        # Проверка Refresh
        print("\n4. Проверка Refresh Token")
        refresh_resp = session.post(urljoin(BASE_URL, "/api/auth/refresh"), cookies={"refresh_token": cookies.get("refresh_token")})
        if refresh_resp.status_code == 200:
            print(f"{GREEN}Refresh token работает успешно{RESET}")
            new_data = refresh_resp.json().get("data", {})
            if new_data.get("access_token"):
                access_token = new_data["access_token"]
                print("   Access token обновлён")
        else:
            print_error("Refresh не удался", refresh_resp)
            return
        time.sleep(1)
        cookies = refresh_resp.cookies

        print("\n5. Изменение данных пользователя")
        set_name_resp = session.patch(urljoin(BASE_URL, "/api/user/display-name"), headers={"Authorization": f"Bearer {access_token}"}, json={"display_name": "MyName2"})
        if set_name_resp.status_code == 200:
            print(f"{GREEN}/api/user/display-name работает корректно{RESET}")
        else:
            print_error("/api/user/display-name вернул ошибку", set_name_resp.text)
            return
        set_lang_resp = session.patch(urljoin(BASE_URL, "/api/user/language"), headers={"Authorization": f"Bearer {access_token}"}, json={"language": "En"})
        if set_lang_resp.status_code == 200:
            print(f"{GREEN}/api/user/language работает корректно{RESET}")
        else:
            print_error("/api/user/language вернул ошибку", set_lang_resp)
            print(set_lang_resp.text)
            return
        time.sleep(1)

        print("\nПолучение информации о пользователе после изменений. Проверка /me")
        me_resp = session.get(urljoin(BASE_URL, "/api/user/me"), headers={"Authorization": f"Bearer {access_token}"})
        if me_resp.status_code == 200:
            print(f"{GREEN}/me работает корректно{RESET}")
            print("   Ответ:", json.dumps(me_resp.json(), indent=2, ensure_ascii=False))
        else:
            print_error("/me вернул ошибку", me_resp)
            return
        time.sleep(1)

        # Logout текущей сессии
        print("\n6. Logout текущей сессии")
        logout_resp = session.post(urljoin(BASE_URL, "/api/auth/logout"), cookies={"refresh_token": cookies.get("refresh_token")})
        if logout_resp.status_code == 200:
            print(f"{GREEN}Logout текущей сессии выполнен{RESET}")
        else:
            print_error("Logout не удался", logout_resp)
        time.sleep(1)

        print("\nПроверка /me - не работает после logout")
        me_resp = session.get(urljoin(BASE_URL, "/api/user/me"), headers={"Authorization": f"Bearer {access_token}"})
        if me_resp.status_code == 200:
            print_error("/me вернул верный ответ после logout, так не должно быть")
            return
        else:
            print(f"{GREEN}/me проведён успешно{RESET}")


def test_logout_all():
    print("=== Тестирование Logout ALL ===\n")
    with httpx.Client() as session:
        print("\nПовторная авторизация для теста logout-all")
        send_resp2 = session.post(urljoin(BASE_URL, "/api/auth/telegram/send-code"))
        if send_resp2.status_code != 200:
            print_error("Не удалось отправить второй код", send_resp2)
            return
        print(send_resp2.text)

        code2 = input("\nВведите новый 6-значный код из Telegram: ").strip()
        verify2_resp = session.post( urljoin(BASE_URL, "/api/auth/telegram/verify"), json={"magic_token": send_resp2.json()['data']['magic_token'], "code": code2})
        if verify2_resp.status_code != 200:
            print_error("Повторная авторизация не удалась", verify2_resp)
            return
        data2 = verify2_resp.json()
        access_token2 = data2.get("data", {}).get("access_token")
        cookies = verify2_resp.cookies
        print(cookies.get("refresh_token")) 
        time.sleep(1)

        print(f"{GREEN}Повторная авторизация прошла успешно{RESET}")

        # Logout All (выход со всех устройств)
        print("\n7. Проверка logout-all")
        logout_all_resp = session.post(urljoin(BASE_URL, "/api/auth/logout-all"), headers={"Authorization": f"Bearer {access_token2}"}, cookies={"refresh_token": cookies.get("refresh_token")})
        if logout_all_resp.status_code == 200:
            print(f"{GREEN}logout-all выполнен успешно{RESET}")
        else:
            print_error("logout-all вернул ошибку", logout_all_resp)
        time.sleep(1)

        print("\nПроверка /me - не работает после logout")
        me_resp = session.get(urljoin(BASE_URL, "/api/user/me"), headers={"Authorization": f"Bearer {access_token}"})
        if me_resp.status_code == 200:
            print_error("/me вернул верный ответ после logout, так не должно быть")
            return
        else:
            print(f"{GREEN}/me проведён успешно{RESET}")            


def test_mail_auth():
    print("=== Тестирование Auth API (email) ===\n")
    with httpx.Client() as session:
        print("1. Отправка кода через Email...")
        send_resp = session.post(urljoin(BASE_URL, "/api/auth/email/send-code"), json={"email": "lukoninmaksim6@gmail.com"})
        if send_resp.status_code != 200:
            print_error("Не удалось отправить код", send_resp)
            return

        print("\n2. Проверка кода (verify)...")
        code = input("\nВведите 6-значный код из Email: ").strip()
        verify_resp = session.post(urljoin(BASE_URL, "/api/auth/email/verify"), json={"magic_token": send_resp.json()['data']['magic_token'], "code": code})
        if verify_resp.status_code != 200:
            print_error("Ошибка верификации", verify_resp)
            return
        time.sleep(1)
        data = verify_resp.json()
        access_token = data.get("data", {}).get("access_token")
        user = data.get("data", {}).get("user", {})
        cookies = verify_resp.cookies      
        if not access_token:
            print_error("Не удалось получить access_token из ответа")
            return

        print(f"{GREEN}Успешная авторизация!{RESET}")
        print(f"   Пользователь: {user.get('display_name') or user.get('email')}")
        print(f"   Access Token получен")

        print("\n3. Получение информации о пользователе. Проверка /me")
        me_resp = session.get(urljoin(BASE_URL, "/api/user/me"), headers={"Authorization": f"Bearer {access_token}"})
        if me_resp.status_code == 200:
            print(f"{GREEN}/me работает корректно{RESET}")
            print("   Ответ:", json.dumps(me_resp.json(), indent=2, ensure_ascii=False))
        else:
            print_error("/me вернул ошибку", me_resp)
            return
        time.sleep(1)

        # Проверка Refresh
        print("\n4. Проверка Refresh Token")
        refresh_resp = session.post(urljoin(BASE_URL, "/api/auth/refresh"), cookies={"refresh_token": cookies.get("refresh_token")})
        if refresh_resp.status_code == 200:
            print(f"{GREEN}Refresh token работает успешно{RESET}")
            new_data = refresh_resp.json().get("data", {})
            if new_data.get("access_token"):
                access_token = new_data["access_token"]
                print("   Access token обновлён")
        else:
            print_error("Refresh не удался", refresh_resp)
            return
        time.sleep(1)
        cookies = refresh_resp.cookies

        # Установка telegram

        print("\n5. Изменение данных пользователя")
        set_tg_resp = session.post(urljoin(BASE_URL, "/api/user/telegram"), headers={"Authorization": f"Bearer {access_token}"})
        if set_tg_resp.status_code == 200:
            print(f"{GREEN}/api/user/telegram работает корректно{RESET}")
        else:
            print("/api/user/telegram вернул ошибку", set_tg_resp.status_code)
            print(set_tg_resp.text)
            return
        time.sleep(1)
        data = set_tg_resp.json()
        print(data)
        code = input("Введите код: ")
        set_tg_resp2 = session.post(urljoin(BASE_URL, "/api/user/telegram/verify"), headers={"Authorization": f"Bearer {access_token}"}, json={"magic_token": data["magic_token"], "code": code})
        if set_tg_resp2.status_code == 200:
            print(f"{GREEN}/api/user/telegram/verify работает корректно{RESET}")
        else:
            print("/api/user/telegram/verify вернул ошибку", set_tg_resp2.status_code)
            print(set_tg_resp2.text)
            return
        time.sleep(1)

        print("\nПолучение информации о пользователе после изменений. Проверка /me")
        me_resp = session.get(urljoin(BASE_URL, "/api/user/me"), headers={"Authorization": f"Bearer {access_token}"})
        if me_resp.status_code == 200:
            print(f"{GREEN}/me работает корректно{RESET}")
            print("   Ответ:", json.dumps(me_resp.json(), indent=2, ensure_ascii=False))
        else:
            print_error("/me вернул ошибку", me_resp)
            return
        time.sleep(1)

        # Logout текущей сессии
        print("\n6. Logout текущей сессии")
        logout_resp = session.post(urljoin(BASE_URL, "/api/auth/logout"), cookies={"refresh_token": cookies.get("refresh_token")})
        if logout_resp.status_code == 200:
            print(f"{GREEN}Logout текущей сессии выполнен{RESET}")
        else:
            print_error("Logout не удался", logout_resp)
        time.sleep(1)

        print("\nПроверка /me - не работает после logout")
        me_resp = session.get(urljoin(BASE_URL, "/api/user/me"), headers={"Authorization": f"Bearer {access_token}"})
        if me_resp.status_code == 200:
            print_error("/me вернул верный ответ после logout, так не должно быть")
            return
        else:
            print(f"{GREEN}/me проведён успешно{RESET}")


if __name__ == "__main__":
    try:
        test_tg_auth()
        test_logout_all()
        test_mail_auth()
        print(f"\n{GREEN}=== Все тесты успешно завершены ==={RESET}")
    except KeyboardInterrupt:
        print(f"\n\nТестирование остановлено пользователем.")
    except Exception as e:
        print(f"{RED}Неожиданная ошибка: {e}{RESET}")

