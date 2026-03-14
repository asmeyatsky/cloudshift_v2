import { Firestore } from "@google-cloud/firestore";

const db = new Firestore();
const COLLECTION_NAME = "users";

interface User {
  userId: string;
  name: string;
  email: string;
}

export async function createUser(user: User): Promise<void> {
  const docRef = db.collection(COLLECTION_NAME).doc(user.userId);
  await docRef.set(user);
}

export async function getUser(userId: string): Promise<User | null> {
  const docRef = db.collection(COLLECTION_NAME).doc(userId);
  const doc = await docRef.get();
  if (!doc.exists) return null;
  return doc.data() as User;
}

export async function deleteUser(userId: string): Promise<void> {
  const docRef = db.collection(COLLECTION_NAME).doc(userId);
  await docRef.delete();
}

export async function queryUsersByEmail(email: string): Promise<User[]> {
  const snapshot = await db
    .collection(COLLECTION_NAME)
    .where("email", "==", email)
    .get();
  return snapshot.docs.map((doc) => doc.data() as User);
}
