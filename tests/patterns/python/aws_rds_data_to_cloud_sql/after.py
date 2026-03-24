from google.cloud.sql.connector import Connector

connector = Connector()
INSTANCE_CONNECTION_NAME = 'my-project:us-central1:mydb'


def run_query(sql: str):
    conn = connector.connect(INSTANCE_CONNECTION_NAME, "pg8000", user="app_user", password="***", db="app")
    cursor = conn.cursor()
    cursor.execute(sql)
    return cursor.fetchall()
