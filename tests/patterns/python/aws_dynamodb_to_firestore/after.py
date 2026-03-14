from google.cloud import firestore

db = firestore.Client()
collection = db.collection('users')


def create_user(user_id, name, email):
    doc_ref = collection.document(user_id)
    doc_ref.set({
        'name': name,
        'email': email,
    })


def get_user(user_id):
    doc_ref = collection.document(user_id)
    doc = doc_ref.get()
    if doc.exists:
        return doc.to_dict()
    return None


def update_user(user_id, name):
    doc_ref = collection.document(user_id)
    doc_ref.update({
        'name': name,
    })


def delete_user(user_id):
    doc_ref = collection.document(user_id)
    doc_ref.delete()


def query_users_by_email(email):
    query = collection.where('email', '==', email)
    docs = query.stream()
    return [doc.to_dict() for doc in docs]
