// CIRCULAR (3-way): Button → Modal → Form → Button
import React from 'react';
import { Modal } from './Modal';

interface ButtonProps {
  children: React.ReactNode;
  onClick?: () => void;
  variant?: 'primary' | 'secondary';
  opensModal?: boolean;
}

export const Button: React.FC<ButtonProps> = ({
  children,
  onClick,
  variant = 'primary',
  opensModal
}) => {
  const [showModal, setShowModal] = React.useState(false);

  return (
    <>
      <button
        className={`btn btn-${variant}`}
        onClick={opensModal ? () => setShowModal(true) : onClick}
      >
        {children}
      </button>
      {opensModal && showModal && (
        <Modal onClose={() => setShowModal(false)}>
          <p>Modal content</p>
        </Modal>
      )}
    </>
  );
};
