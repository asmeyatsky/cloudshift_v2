import firebase_admin
from firebase_admin import auth

firebase_admin.initialize_app()


def get_user(username):
    return auth.get_user_by_email(username)


def set_user_password(username, password):
    auth.update_user(username, password=password)
