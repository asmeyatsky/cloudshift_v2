from google.cloud.sql.connector import Connector

connector = Connector()
INSTANCE_CONNECTION_NAME = "my-project:us-central1:mydb"


def fetch_users():
    with connector.connect(INSTANCE_CONNECTION_NAME, "pg8000", user="user", password="***", db="mydb") as conn:
        cur = conn.cursor()
        cur.execute("SELECT id, name FROM users WHERE active = 1")
        return cur.fetchall()
