// Non-circular: page using hooks and components
import React from 'react';
import { useAuth } from '../hooks/useAuth';
import { Card } from '../components/Card';
import { Button } from '../components/Button';

export const ProfilePage: React.FC = () => {
  const { user, logout } = useAuth();

  if (!user) {
    return <p>Please log in</p>;
  }

  return (
    <div className="profile-page">
      <Card title="Profile">
        <p>Name: {user.name}</p>
        <p>Email: {user.email}</p>
        <Button onClick={logout} variant="secondary">Logout</Button>
      </Card>
    </div>
  );
};
