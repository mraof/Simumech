package com.mraof.simumech.markov;

import java.io.BufferedReader;
import java.io.FileNotFoundException;
import java.io.FileReader;
import java.io.IOException;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.Random;
import java.util.concurrent.locks.ReadWriteLock;
import java.util.concurrent.locks.ReentrantReadWriteLock;

import com.mraof.simumech.Util;

public class MarkovChain 
{
	Random rand = new Random();
	//key is pair of two words
	HashMap<String, ArrayList<String>> wordPairs = new HashMap<String, ArrayList<String>>();
	//key is a single word, used if pair can't be found
	HashMap<String, ArrayList<String>> words = new HashMap<String, ArrayList<String>>();
	ArrayList<String> lines = new ArrayList<String>();

	ReadWriteLock lock = new ReentrantReadWriteLock();

	public MarkovChain() 
	{
		try {
			BufferedReader reader = new BufferedReader(new FileReader("lines.txt"));
			String line;
			while((line = reader.readLine()) != null)
			{
				addLine(line);
				System.out.printf("Added %d lines\r", lines.size());
			}
			System.out.println();
			reader.close();
		} catch (FileNotFoundException e) {e.printStackTrace();} catch (IOException e) {e.printStackTrace();}
		System.out.printf("Loaded %d lines, %d words, %d word pairs\n", lines.size(), words.size(), wordPairs.size());
	}

	public void addLine(String line)
	{
		lock.writeLock().lock();
		{
			lines.add(line);
			ArrayList<String> currentWords = Util.split(line);
			String previousWord = "";
			for(int i = 0; i < currentWords.size() - 1; i++)
			{
				String currentWord = currentWords.get(i).toLowerCase();
				String nextWord = currentWords.get(i + 1).toLowerCase();
				String pair = previousWord + " " + currentWord;
				ArrayList pairList = wordPairs.get(pair);
				if(pairList == null)
					pairList = new ArrayList<String>();
				pairList.add(nextWord);
				wordPairs.put(pair, pairList);

				ArrayList wordList = words.get(currentWord);
				if(wordList == null)
					wordList = new ArrayList<String>();
				wordList.add(nextWord);
				words.put(currentWord, wordList);
				
				previousWord = currentWord;
//				System.out.println("\"" + currentWords.get(i) + "\",");
			}
		}
		lock.writeLock().unlock();
	}

	public String reply(String inputString)
	{
		ArrayList<String> currentLines;
		ArrayList<String> currentWords = new ArrayList<String>();
		ArrayList<String> index = new ArrayList<String>();
		ArrayDeque<String> sentence = new ArrayDeque<String>();
		String replyString = "";
		boolean done = false;

		if(inputString.isEmpty())
		{
			System.out.println("Empty input string");
			return "";
		}

		String message = inputString;
		currentLines = Util.split(message, ". ");

		for(int i = 0; i < currentLines.size(); i++)
			currentWords.addAll(Util.split(currentLines.get(i)));
		if(currentWords.isEmpty())
		{
			System.out.println("Input string contained now words");
			return "";
		}
		String previousWord = "";
		for(int i = 0; i < currentWords.size(); i++)
		{
			String currentWord = currentWords.get(i).toLowerCase();
			String pairKey = previousWord + " " + currentWord;
			ArrayList<String> list = wordPairs.get(pairKey);
			if(list != null && rand.nextDouble() > .1)
			{
				String word = list.get(rand.nextInt(list.size()));
				System.out.println("Adding " + word + " to sentence from pair " + pairKey);
				sentence.add(word);
				break;
			}
			else if(rand.nextDouble() > (1 / (currentWords.size() - i + 1) + .2))
			{
				String key = currentWord;
				list = words.get(key);
				if(list != null)
				{
					String word = list.get(rand.nextInt(list.size()));
					System.out.println("Adding " + word + " to sentence from word " + key);
					break;
				}
			}
			previousWord = currentWord;

		}
		if(sentence.isEmpty())
			sentence.add(currentWords.get(0));
		
		for(int size = sentence.size() - 1; size < sentence.size(); )
		{
			size = sentence.size();
			String currentWord = sentence.getLast();
			String key = previousWord + " " + currentWord;
			ArrayList<String> list = wordPairs.get(key);
			if(list != null && rand.nextDouble() > .1)
			{
				String word = list.get(rand.nextInt(list.size()));
				System.out.println("Adding " + word + " to sentence from pair " + key);
				sentence.add(word);
			}
			else /*if(rand.nextDouble() > (1 / (sentence.size() + .5) + .2))*/
			{
				key = currentWord;
				list = words.get(key);
				if(list != null)
				{
					String word = list.get(rand.nextInt(list.size()));
					System.out.println("Adding " + word + " to sentence from word " + key);
					sentence.add(word);
				}
			}
			previousWord = currentWord;

		}

		replyString = sentence.pollFirst();
		replyString = replyString.substring(0, 1).toUpperCase() + replyString.substring(1);
		for(String replyWord : sentence)
			replyString += " " + replyWord;
		return replyString;
	}


	private double[] multiplyList(ArrayList<Double> a, ArrayList<Double> b) 
	{
		if (a.size() != b.size())
			return null;

		double[] results = new double[a.size()];
		for (int i = 0; i < results.length; i++)
			results[i] = a.get(i) * b.get(i);

		return results;
	}

	private double[] multiplyMatrix(ArrayList<Double> a, ArrayList<ArrayList<Double>> b) 
	{
		if (b.size() == 0 || a.size() != b.get(0).size())
			return null;

		double[] results = new double[b.size()];
		for (int i = 0; i < results.length; i++) {
			results[i] = 0;
			for (int j = 0; j < a.size(); j++)
				results[i] += a.get( j ) * b.get(i).get( j );
		}

		return results;
	}

	private int choose(double[] probabilities) {
		double probability = rand.nextDouble();
		for (int i = 0; i < probabilities.length; i++)
			if (probabilities[i] < probability)
				return i;
			else
				probability -= probabilities[i];
		return -1;
	}
}

