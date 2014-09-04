package com.mraof.simumech.markov;

public class Word 
{
	String string;
	private int count = 1;
	
	public Word(String string) 
	{
		this.string = string;
	}
	
	public int increment()
	{
		count++;
		return count;
	}
	
	public int getCount()
	{
		return count;
	}
	
	@Override
	public String toString() 
	{
		return string;
	}
}
