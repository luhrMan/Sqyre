package editor

import (
	"Sqyre/internal/models"
	"log"
)

func ProgramItemRepo(p *models.Program) models.ItemRepositoryInterface {
	repo, err := p.ItemRepo()
	if err != nil {
		log.Printf("item repo: %v", err)
		return nil
	}
	return repo
}

func ProgramPointRepo(p *models.Program, resolutionKey string) models.PointRepositoryInterface {
	repo, err := p.PointRepo(resolutionKey)
	if err != nil {
		log.Printf("point repo: %v", err)
		return nil
	}
	return repo
}

func ProgramSearchAreaRepo(p *models.Program, resolutionKey string) models.SearchAreaRepositoryInterface {
	repo, err := p.SearchAreaRepo(resolutionKey)
	if err != nil {
		log.Printf("search area repo: %v", err)
		return nil
	}
	return repo
}

func ProgramMaskRepo(p *models.Program) models.MaskRepositoryInterface {
	repo, err := p.MaskRepo()
	if err != nil {
		log.Printf("mask repo: %v", err)
		return nil
	}
	return repo
}
