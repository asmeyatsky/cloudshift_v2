from google.cloud import firestore

db = firestore.Client()
collection = db.collection("users")


def get_user(user_id):
    doc = collection.document(user_id).get()
    return doc.to_dict() if doc.exists else None


def upsert_user(doc):
    collection.document(doc["id"]).set(doc)
