"""
Boundary management endpoints.

Handles boundary CRUD operations.
**TEMPORARY**: Uses in-memory storage for demo purposes.
Week 3 will implement proper database storage.
"""

import logging

from fastapi import APIRouter, Depends, HTTPException, status

from ..auth import User, get_current_user
from ..models import DesignBoundary

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/boundaries", tags=["boundaries"])

# TEMPORARY: In-memory storage for boundaries (demo purposes only)
# Week 3 will replace this with database storage
_boundaries_store: dict[str, DesignBoundary] = {}


@router.get("", response_model=list[DesignBoundary], status_code=status.HTTP_200_OK)
async def list_boundaries(current_user: User = Depends(get_current_user)) -> list[DesignBoundary]:
    """
    List all design boundaries.

    **TEMPORARY**: Returns boundaries from in-memory store.
    Week 3 will implement database storage and retrieval.

    Returns:
        List of DesignBoundary objects
    """
    logger.info(f"Listing boundaries (in-memory store: {len(_boundaries_store)} boundaries)")
    return list(_boundaries_store.values())


@router.post("", response_model=DesignBoundary, status_code=status.HTTP_200_OK)
async def create_boundary(boundary: DesignBoundary, current_user: User = Depends(get_current_user)) -> DesignBoundary:
    """
    Create a new design boundary.

    **TEMPORARY**: Stores boundary in in-memory store.
    Week 3 will implement database storage.

    Args:
        boundary: DesignBoundary object to create

    Returns:
        Created DesignBoundary

    Raises:
        HTTPException: If boundary with same ID already exists
    """
    if boundary.id in _boundaries_store:
        logger.warning(f"Attempted to create duplicate boundary: {boundary.id}")
        raise HTTPException(
            status_code=status.HTTP_409_CONFLICT,
            detail=f"Boundary with id '{boundary.id}' already exists"
        )

    _boundaries_store[boundary.id] = boundary
    logger.info(f"Created boundary: {boundary.id} ({boundary.name})")
    return boundary


@router.get("/{boundary_id}", response_model=DesignBoundary, status_code=status.HTTP_200_OK)
async def get_boundary(boundary_id: str, current_user: User = Depends(get_current_user)) -> DesignBoundary:
    """
    Get a specific boundary by ID.

    **TEMPORARY**: Retrieves from in-memory store.
    Week 3 will implement database retrieval.

    Args:
        boundary_id: Boundary identifier

    Returns:
        DesignBoundary object

    Raises:
        HTTPException: If boundary not found
    """
    if boundary_id not in _boundaries_store:
        logger.warning(f"Boundary not found: {boundary_id}")
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Boundary '{boundary_id}' not found"
        )

    return _boundaries_store[boundary_id]


@router.delete("/{boundary_id}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_boundary(boundary_id: str, current_user: User = Depends(get_current_user)) -> None:
    """
    Delete a boundary by ID.

    **TEMPORARY**: Deletes from in-memory store.
    Week 3 will implement database deletion.

    Args:
        boundary_id: Boundary identifier

    Raises:
        HTTPException: If boundary not found
    """
    if boundary_id not in _boundaries_store:
        logger.warning(f"Attempted to delete non-existent boundary: {boundary_id}")
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Boundary '{boundary_id}' not found"
        )

    del _boundaries_store[boundary_id]
    logger.info(f"Deleted boundary: {boundary_id}")
