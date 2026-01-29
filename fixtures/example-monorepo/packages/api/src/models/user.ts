// Non-circular: leaf node (no imports from project)
export interface User {
  id: string;
  name: string;
  email: string;
}

export interface UserCreateInput {
  name: string;
  email: string;
}
