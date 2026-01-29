// Non-circular: base type definition
export interface User {
  id: string;
  name: string;
  email: string;
  createdAt: Date;
}

export interface UserProfile extends User {
  avatar?: string;
  bio?: string;
}
